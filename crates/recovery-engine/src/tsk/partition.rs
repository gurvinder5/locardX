use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};

/// Information about a detected partition in an evidence disk image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub index: usize,
    pub partition_type: String,
    pub start_lba: u64,
    pub end_lba: u64,
    pub sector_count: u64,
    pub start_byte_offset: u64,
    pub size_bytes: u64,
    pub filesystem_hint: Option<String>,
}

const SECTOR_SIZE: u64 = 512;

/// Scans an evidence image stream for GPT or MBR partition tables.
/// Returns detected partitions, or a whole-disk fallback partition if none exist.
pub fn scan_partitions<R: Read + Seek>(reader: &mut R, total_bytes: u64) -> Vec<PartitionInfo> {
    // 1. Try GPT first (LBA 1)
    if let Ok(gpt_partitions) = scan_gpt(reader, total_bytes) {
        if !gpt_partitions.is_empty() {
            return gpt_partitions;
        }
    }

    // 2. Try MBR (LBA 0)
    if let Ok(mbr_partitions) = scan_mbr(reader, total_bytes) {
        if !mbr_partitions.is_empty() {
            return mbr_partitions;
        }
    }

    // 3. Fallback to whole-image single partition (e.g. unpartitioned volume image)
    vec![PartitionInfo {
        index: 0,
        partition_type: "Raw Volume".to_string(),
        start_lba: 0,
        end_lba: total_bytes / SECTOR_SIZE,
        sector_count: total_bytes / SECTOR_SIZE,
        start_byte_offset: 0,
        size_bytes: total_bytes,
        filesystem_hint: detect_vbr_filesystem_hint(reader, 0),
    }]
}

fn scan_gpt<R: Read + Seek>(
    reader: &mut R,
    total_bytes: u64,
) -> Result<Vec<PartitionInfo>, std::io::Error> {
    if total_bytes < SECTOR_SIZE * 34 {
        return Ok(Vec::new());
    }

    // Read GPT Header at LBA 1 (offset 512)
    reader.seek(SeekFrom::Start(SECTOR_SIZE))?;
    let mut header = [0u8; 92];
    reader.read_exact(&mut header)?;

    // Signature: "EFI PART" (0x5452415020494645)
    if &header[0..8] != b"EFI PART" {
        return Ok(Vec::new());
    }

    let partition_entry_lba = u64::from_le_bytes(header[72..80].try_into().unwrap());
    let num_entries = u32::from_le_bytes(header[80..84].try_into().unwrap()) as usize;
    let entry_size = u32::from_le_bytes(header[84..88].try_into().unwrap()) as usize;

    if entry_size < 128 || num_entries == 0 || num_entries > 256 {
        return Ok(Vec::new());
    }

    reader.seek(SeekFrom::Start(partition_entry_lba * SECTOR_SIZE))?;
    let mut partitions = Vec::new();
    let mut entry_buf = vec![0u8; entry_size];

    for idx in 0..num_entries {
        if reader.read_exact(&mut entry_buf).is_err() {
            break;
        }

        // Empty partition type GUID is all zeros
        if entry_buf[0..16].iter().all(|&b| b == 0) {
            continue;
        }

        let start_lba = u64::from_le_bytes(entry_buf[32..40].try_into().unwrap());
        let end_lba = u64::from_le_bytes(entry_buf[40..48].try_into().unwrap());

        if end_lba >= start_lba {
            let sector_count = end_lba - start_lba + 1;
            let start_byte_offset = start_lba * SECTOR_SIZE;
            let size_bytes = sector_count * SECTOR_SIZE;

            // Name in UTF-16LE
            let mut name_u16 = Vec::new();
            for chunk in entry_buf[56..128.min(entry_size)].chunks_exact(2) {
                let code = u16::from_le_bytes([chunk[0], chunk[1]]);
                if code == 0 {
                    break;
                }
                name_u16.push(code);
            }
            let part_name = String::from_utf16_lossy(&name_u16);
            let part_type = if part_name.trim().is_empty() {
                format!("GPT Partition {}", idx + 1)
            } else {
                part_name.trim().to_string()
            };

            let hint = detect_vbr_filesystem_hint(reader, start_byte_offset);

            partitions.push(PartitionInfo {
                index: idx + 1,
                partition_type: part_type,
                start_lba,
                end_lba,
                sector_count,
                start_byte_offset,
                size_bytes,
                filesystem_hint: hint,
            });
        }
    }

    Ok(partitions)
}

fn scan_mbr<R: Read + Seek>(
    reader: &mut R,
    total_bytes: u64,
) -> Result<Vec<PartitionInfo>, std::io::Error> {
    if total_bytes < SECTOR_SIZE {
        return Ok(Vec::new());
    }

    reader.seek(SeekFrom::Start(0))?;
    let mut sector = [0u8; 512];
    reader.read_exact(&mut sector)?;

    if sector[510] != 0x55 || sector[511] != 0xAA {
        return Ok(Vec::new());
    }

    let mut partitions = Vec::new();

    // 4 partition records starting at 0x1BE (446)
    for i in 0..4 {
        let offset = 446 + i * 16;
        let ptype = sector[offset + 4];
        if ptype == 0x00 || ptype == 0xEE {
            // 0x00 = unused, 0xEE = GPT Protective MBR (handled by GPT scanner)
            continue;
        }

        let start_lba =
            u32::from_le_bytes(sector[offset + 8..offset + 12].try_into().unwrap()) as u64;
        let sector_count =
            u32::from_le_bytes(sector[offset + 12..offset + 16].try_into().unwrap()) as u64;

        if sector_count > 0 && start_lba * SECTOR_SIZE < total_bytes {
            let start_byte_offset = start_lba * SECTOR_SIZE;
            let size_bytes = sector_count * SECTOR_SIZE;
            let type_str = match ptype {
                0x07 => "NTFS/exFAT",
                0x0B | 0x0C => "FAT32",
                0x83 => "Linux ext4",
                _other => "MBR Type",
            };

            let hint = detect_vbr_filesystem_hint(reader, start_byte_offset);

            partitions.push(PartitionInfo {
                index: i + 1,
                partition_type: format!("{} (0x{:02X})", type_str, ptype),
                start_lba,
                end_lba: start_lba + sector_count - 1,
                sector_count,
                start_byte_offset,
                size_bytes,
                filesystem_hint: hint,
            });
        }
    }

    Ok(partitions)
}

/// Peeks at volume boot record / superblock signatures to identify filesystem.
pub fn detect_vbr_filesystem_hint<R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
) -> Option<String> {
    let mut buf = [0u8; 1024];
    if reader.seek(SeekFrom::Start(start_offset)).is_err() {
        return None;
    }
    if reader.read_exact(&mut buf).is_err() {
        return None;
    }

    // NTFS check: OEM ID at bytes 3..11 is "NTFS    "
    if &buf[3..11] == b"NTFS    " {
        return Some("NTFS".to_string());
    }

    // FAT32 check: "FAT32   " at offset 82..90
    if &buf[82..90] == b"FAT32   " {
        return Some("FAT32".to_string());
    }

    // exFAT check: "EXFAT   " at offset 3..11
    if &buf[3..11] == b"EXFAT   " {
        return Some("exFAT".to_string());
    }

    // ext4 check: Superblock at start_offset + 1024, magic 0xEF53 at offset 56..58 (i.e. byte 1080)
    let mut sb_buf = [0u8; 1024];
    if reader.seek(SeekFrom::Start(start_offset + 1024)).is_ok()
        && reader.read_exact(&mut sb_buf).is_ok()
        && sb_buf[56] == 0x53
        && sb_buf[57] == 0xEF
    {
        return Some("ext4".to_string());
    }

    None
}
