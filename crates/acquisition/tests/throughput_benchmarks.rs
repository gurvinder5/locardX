use locardx_acquisition::engine::AcquisitionEngine;
use locardx_acquisition::hasher::StreamingSha256Hasher;
use locardx_acquisition::models::AcquisitionFailureReason;
use locardx_acquisition::platform::ReadOnlyDeviceStream;
use locardx_acquisition::writer::RawImageWriter;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;
use std::time::Instant;

/// Mock in-memory physical stream simulating a high-speed disk or USB device.
struct SyntheticDeviceStream {
    pattern_chunk: Vec<u8>,
    total_size: u64,
    current_offset: u64,
}

impl SyntheticDeviceStream {
    fn new(total_size: u64, seed_pattern_size: usize) -> Self {
        let mut pattern_chunk = vec![0u8; seed_pattern_size];
        for (i, b) in pattern_chunk.iter_mut().enumerate() {
            *b = ((i * 31 + 17) & 0xFF) as u8;
        }
        Self {
            pattern_chunk,
            total_size,
            current_offset: 0,
        }
    }
}

impl ReadOnlyDeviceStream for SyntheticDeviceStream {
    fn read_chunk(&mut self, buffer: &mut [u8]) -> Result<usize, AcquisitionFailureReason> {
        if self.current_offset >= self.total_size {
            return Ok(0);
        }
        let remaining = (self.total_size - self.current_offset) as usize;
        let to_read = buffer.len().min(remaining);
        if to_read == 0 {
            return Ok(0);
        }

        let pat_len = self.pattern_chunk.len();
        let mut filled = 0;
        while filled < to_read {
            let chunk_fill = (to_read - filled).min(pat_len);
            buffer[filled..filled + chunk_fill].copy_from_slice(&self.pattern_chunk[..chunk_fill]);
            filled += chunk_fill;
        }

        self.current_offset += to_read as u64;
        Ok(to_read)
    }

    fn total_bytes(&self) -> u64 {
        self.total_size
    }

    fn sector_size(&self) -> u32 {
        512
    }

    fn current_offset(&self) -> u64 {
        self.current_offset
    }
}

struct BufferMessage {
    buf: Vec<u8>,
    valid_len: usize,
}

#[test]
fn benchmark_all_chunk_sizes() {
    let test_data_size: u64 = 64 * 1024 * 1024; // 64 MiB per test
    let chunk_sizes = [
        512 * 1024,       // 512 KiB
        1024 * 1024,      // 1 MiB
        4 * 1024 * 1024,  // 4 MiB
        8 * 1024 * 1024,  // 8 MiB
        16 * 1024 * 1024, // 16 MiB
        32 * 1024 * 1024, // 32 MiB
    ];

    println!("\n================================================================================");
    println!("              LOCARDX ACQUISITION PERFORMANCE BENCHMARK SUITE");
    println!(
        "              Dataset size per run: 64 MiB ({} bytes)",
        test_data_size
    );
    println!("================================================================================");

    // 1. Benchmark Raw SHA-256 Throughput
    println!("\n--- 1. Pure SHA-256 Hashing Throughput ---");
    for &chunk_size in &chunk_sizes {
        let pattern = vec![0xA5u8; chunk_size];
        let chunks_count = (test_data_size as usize) / chunk_size;

        let start = Instant::now();
        let mut hasher = StreamingSha256Hasher::new();
        for _ in 0..chunks_count {
            hasher.update(&pattern);
        }
        let (digest, bytes) = hasher.finalize();
        let elapsed = start.elapsed().as_secs_f64();
        let mib_per_sec = (bytes as f64 / (1024.0 * 1024.0)) / elapsed;

        println!(
            "  Chunk: {:>6} KiB | Time: {:>6.2} ms | Throughput: {:>7.2} MiB/s | Hash: {:.8}...",
            chunk_size / 1024,
            elapsed * 1000.0,
            mib_per_sec,
            digest
        );
    }

    // 2. Benchmark Destination Writing (OS filesystem caching / SSD)
    println!("\n--- 2. Sequential Destination Disk Write Throughput ---");
    for &chunk_size in &chunk_sizes {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_file = temp_dir.path().join("bench_write.raw");
        let pattern = vec![0x5Au8; chunk_size];
        let chunks_count = (test_data_size as usize) / chunk_size;

        let start = Instant::now();
        let mut writer = RawImageWriter::create(&dest_file).unwrap();
        for _ in 0..chunks_count {
            writer.write_chunk(&pattern).unwrap();
        }
        let written = writer.flush_and_sync().unwrap();
        let (_, promoted_path) = writer.finalize_to_destination().unwrap();
        let elapsed = start.elapsed().as_secs_f64();
        let mib_per_sec = (written as f64 / (1024.0 * 1024.0)) / elapsed;

        assert_eq!(written, test_data_size);
        assert!(promoted_path.exists());

        println!(
            "  Chunk: {:>6} KiB | Time: {:>6.2} ms | Throughput: {:>7.2} MiB/s",
            chunk_size / 1024,
            elapsed * 1000.0,
            mib_per_sec
        );
    }

    // 3. Benchmark Existing Sequential Acquisition
    println!("\n--- 3. Sequential Acquisition Pipeline (Existing) ---");
    for &chunk_size in &chunk_sizes {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_file = temp_dir
            .path()
            .join(format!("bench_seq_{}k.raw", chunk_size / 1024));
        let stream = Box::new(SyntheticDeviceStream::new(test_data_size, 64 * 1024));

        let start = Instant::now();
        let output = AcquisitionEngine::execute(
            &format!("bench-seq-{}", chunk_size),
            stream,
            &dest_file,
            chunk_size,
            test_data_size,
            None,
            None,
        )
        .expect("Sequential acquisition failed");

        let elapsed = start.elapsed().as_secs_f64();
        let mib_per_sec = (output.bytes_acquired as f64 / (1024.0 * 1024.0)) / elapsed;

        assert_eq!(output.bytes_acquired, test_data_size);

        println!(
            "  Chunk: {:>6} KiB | Time: {:>6.2} ms | Throughput: {:>7.2} MiB/s | SHA: {:.8}...",
            chunk_size / 1024,
            elapsed * 1000.0,
            mib_per_sec,
            output.image_sha256
        );
    }

    // 4. Benchmark Overlapped Bounded Producer-Consumer Pipeline (Double Buffering)
    println!("\n--- 4. Bounded Producer-Consumer Overlapped Pipeline (Double Buffer) ---");
    for &chunk_size in &chunk_sizes {
        let temp_dir = tempfile::tempdir().unwrap();
        let dest_file = temp_dir
            .path()
            .join(format!("bench_pipe_{}k.raw", chunk_size / 1024));
        let mut stream = Box::new(SyntheticDeviceStream::new(test_data_size, 64 * 1024));

        let start = Instant::now();
        let mut writer = RawImageWriter::create(&dest_file).unwrap();
        let mut hasher = StreamingSha256Hasher::new();

        // 2 pre-allocated reusable buffers: double-buffering
        let (free_tx, free_rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = sync_channel(2);
        let (work_tx, work_rx): (
            SyncSender<Option<BufferMessage>>,
            Receiver<Option<BufferMessage>>,
        ) = sync_channel(2);

        // Pre-allocate 2 buffers upfront (zero allocations inside loop)
        free_tx.send(vec![0u8; chunk_size]).unwrap();
        free_tx.send(vec![0u8; chunk_size]).unwrap();

        let writer_free_tx = free_tx.clone();
        let writer_handle = thread::spawn(move || {
            let mut _total_written = 0u64;
            while let Ok(msg_opt) = work_rx.recv() {
                match msg_opt {
                    Some(msg) => {
                        writer.write_chunk(&msg.buf[..msg.valid_len]).unwrap();
                        _total_written += msg.valid_len as u64;
                        // Return buffer to free pool for reuse
                        let _ = writer_free_tx.send(msg.buf);
                    }
                    None => break, // EOF signal
                }
            }
            let written = writer.flush_and_sync().unwrap();
            let (_, path) = writer.finalize_to_destination().unwrap();
            (written, path)
        });

        // Reader & Hasher loop on caller thread
        let mut total_read = 0u64;
        while total_read < test_data_size {
            let mut buf = free_rx.recv().unwrap();
            let n = stream.read_chunk(&mut buf[..chunk_size]).unwrap();
            if n == 0 {
                let _ = free_tx.send(buf);
                break;
            }
            hasher.update(&buf[..n]);
            total_read += n as u64;
            work_tx
                .send(Some(BufferMessage { buf, valid_len: n }))
                .unwrap();
        }
        work_tx.send(None).unwrap(); // Signal EOF

        let (written, path) = writer_handle.join().unwrap();
        let (hash, hash_bytes) = hasher.finalize();
        let elapsed = start.elapsed().as_secs_f64();
        let mib_per_sec = (written as f64 / (1024.0 * 1024.0)) / elapsed;

        assert_eq!(written, test_data_size);
        assert_eq!(hash_bytes, test_data_size);
        assert!(path.exists());

        println!(
            "  Chunk: {:>6} KiB | Time: {:>6.2} ms | Throughput: {:>7.2} MiB/s | SHA: {:.8}...",
            chunk_size / 1024,
            elapsed * 1000.0,
            mib_per_sec,
            hash
        );
    }

    println!("================================================================================\n");
}
