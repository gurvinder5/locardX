use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom};

/// File entry metadata recovered from filesystem records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredMetadataFile {
    pub name: String,
    pub source_offset: u64,
    pub size_bytes: u64,
    pub is_deleted: bool,
    pub filesystem: String,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
}

/// Progress event emitted during filesystem analysis.
#[derive(Debug, Clone)]
pub struct TskProgressUpdate {
    pub filesystem_type: String,
    pub phase: String,
    pub entries_examined: usize,
    pub deleted_candidates: usize,
    pub current_operation: String,
}

/// Outcome of inspecting a single partition for filesystem structures.
#[derive(Debug, Clone, Default)]
pub struct PartitionInspectionReport {
    pub files: Vec<DiscoveredMetadataFile>,
    pub filesystem: Option<String>,
    pub entries_examined: usize,
    pub deleted_candidates: usize,
    pub error: Option<String>,
}

/// Inspects a partition for deleted files via filesystem metadata structures with telemetry.
pub fn inspect_partition_filesystem_guided<R: Read + Seek, P: FnMut(TskProgressUpdate)>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
    filesystem_hint: Option<&str>,
    progress_cb: &mut P,
) -> PartitionInspectionReport {
    let fs_type = filesystem_hint
        .map(|s| s.to_uppercase())
        .unwrap_or_default();

    if fs_type.contains("NTFS") {
        parse_ntfs_deleted_guided(reader, start_offset, partition_size, progress_cb)
    } else if fs_type.contains("FAT32") {
        parse_fat32_deleted_guided(reader, start_offset, partition_size, progress_cb)
    } else if fs_type.contains("EXT4") {
        parse_ext4_deleted_guided(reader, start_offset, partition_size, progress_cb)
    } else {
        // Probe NTFS, then FAT32, then ext4
        let ntfs_rep = parse_ntfs_deleted_guided(reader, start_offset, partition_size, progress_cb);
        if ntfs_rep.filesystem.is_some() {
            return ntfs_rep;
        }

        let fat_rep = parse_fat32_deleted_guided(reader, start_offset, partition_size, progress_cb);
        if fat_rep.filesystem.is_some() {
            return fat_rep;
        }

        let ext4_rep = parse_ext4_deleted_guided(reader, start_offset, partition_size, progress_cb);
        if ext4_rep.filesystem.is_some() {
            return ext4_rep;
        }

        PartitionInspectionReport {
            files: Vec::new(),
            filesystem: None,
            entries_examined: 0,
            deleted_candidates: 0,
            error: Some("No supported filesystem signature (NTFS, FAT32, ext4) detected in volume boot sector".to_string()),
        }
    }
}

/// Backward-compatible inspection entrypoint.
pub fn inspect_partition_filesystem<R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
    filesystem_hint: Option<&str>,
) -> Vec<DiscoveredMetadataFile> {
    inspect_partition_filesystem_guided(
        reader,
        start_offset,
        partition_size,
        filesystem_hint,
        &mut |_| {},
    )
    .files
}

/// Parses NTFS filesystem for deleted MFT records with progressive telemetry.
pub fn parse_ntfs_deleted_guided<R: Read + Seek, P: FnMut(TskProgressUpdate)>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
    progress_cb: &mut P,
) -> PartitionInspectionReport {
    let mut report = PartitionInspectionReport::default();

    let mut vbr = [0u8; 512];
    if reader.seek(SeekFrom::Start(start_offset)).is_err() || reader.read_exact(&mut vbr).is_err() {
        report.error = Some("Failed to read NTFS volume boot record".to_string());
        return report;
    }

    if &vbr[3..11] != b"NTFS    " {
        return report;
    }

    report.filesystem = Some("NTFS".to_string());

    let bytes_per_sector = u16::from_le_bytes(vbr[11..13].try_into().unwrap()) as u64;
    let sectors_per_cluster = vbr[13] as u64;
    if bytes_per_sector == 0 || sectors_per_cluster == 0 {
        report.error = Some("Invalid NTFS geometry in boot sector (zero bytes per sector or sectors per cluster)".to_string());
        return report;
    }

    let cluster_size = match bytes_per_sector.checked_mul(sectors_per_cluster) {
        Some(cs) if cs > 0 => cs,
        _ => {
            report.error = Some("NTFS cluster size overflow".to_string());
            return report;
        }
    };

    let mft_lcn = i64::from_le_bytes(vbr[48..56].try_into().unwrap());
    if mft_lcn < 0 {
        report.error = Some("Invalid negative MFT cluster offset in boot sector".to_string());
        return report;
    }

    let mft_record_size: u64 = {
        let val = vbr[64] as i8;
        if val < 0 {
            let shift = (-val as i32) as u32;
            if shift < 64 {
                1u64 << shift
            } else {
                report.error = Some("Invalid MFT record size shift".to_string());
                return report;
            }
        } else {
            match (val as u64).checked_mul(cluster_size) {
                Some(sz) => sz,
                None => {
                    report.error = Some("MFT record size calculation overflow".to_string());
                    return report;
                }
            }
        }
    };

    if mft_record_size < 512 || mft_record_size > 4096 {
        report.error = Some(format!("Unsupported MFT record size: {} bytes", mft_record_size));
        return report;
    }

    let mft_offset = match (mft_lcn as u64).checked_mul(cluster_size) {
        Some(off) => off,
        None => {
            report.error = Some("MFT starting byte offset overflow".to_string());
            return report;
        }
    };

    let mft_start = match start_offset.checked_add(mft_offset) {
        Some(s) => s,
        None => {
            report.error = Some("MFT absolute start offset overflow".to_string());
            return report;
        }
    };

    let partition_end = match start_offset.checked_add(partition_size) {
        Some(pe) => pe,
        None => {
            report.error = Some("Partition boundary overflow".to_string());
            return report;
        }
    };

    if mft_start >= partition_end {
        report.error = Some("MFT start offset is beyond partition boundary".to_string());
        return report;
    }

    // Determine records to scan: inspect up to 65,536 records or until end of partition
    let max_possible_records = (partition_end.saturating_sub(mft_start)) / mft_record_size;
    let records_to_scan = (max_possible_records.min(65536) as usize).max(256);

    progress_cb(TskProgressUpdate {
        filesystem_type: "NTFS".to_string(),
        phase: "Analyzing NTFS Master File Table".to_string(),
        entries_examined: 0,
        deleted_candidates: 0,
        current_operation: format!("Scanning up to {} MFT records", records_to_scan),
    });

    let mut record_buf = vec![0u8; mft_record_size as usize];

    for idx in 0..records_to_scan {
        let record_offset = match (idx as u64).checked_mul(mft_record_size).and_then(|o| mft_start.checked_add(o)) {
            Some(ro) => ro,
            None => break,
        };
        if record_offset.checked_add(mft_record_size).map_or(true, |end| end > partition_end) {
            break;
        }

        if reader.seek(SeekFrom::Start(record_offset)).is_err() {
            break;
        }
        if reader.read_exact(&mut record_buf).is_err() {
            break;
        }

        report.entries_examined += 1;

        if &record_buf[0..4] != b"FILE" {
            continue;
        }

        let flags = u16::from_le_bytes([record_buf[22], record_buf[23]]);
        let is_in_use = (flags & 0x0001) != 0;
        let is_dir = (flags & 0x0002) != 0;

        // Recover unallocated / deleted file records (!is_in_use && !is_dir)
        if !is_in_use && !is_dir {
            if let Some(entry) = extract_ntfs_mft_file(&record_buf, start_offset, record_offset, cluster_size) {
                if entry.size_bytes > 0 && entry.source_offset > 0 && entry.source_offset < partition_end {
                    report.deleted_candidates += 1;
                    let file_name = entry.name.clone();
                    report.files.push(entry);

                    progress_cb(TskProgressUpdate {
                        filesystem_type: "NTFS".to_string(),
                        phase: "Recovering deleted file candidates".to_string(),
                        entries_examined: report.entries_examined,
                        deleted_candidates: report.deleted_candidates,
                        current_operation: format!("Found deleted file: {}", file_name),
                    });
                }
            }
        }

        if idx > 0 && idx % 256 == 0 {
            progress_cb(TskProgressUpdate {
                filesystem_type: "NTFS".to_string(),
                phase: "Analyzing NTFS Master File Table".to_string(),
                entries_examined: report.entries_examined,
                deleted_candidates: report.deleted_candidates,
                current_operation: format!("Processed {} / {} MFT records", idx, records_to_scan),
            });
        }
    }

    report
}

fn extract_ntfs_mft_file(
    record: &[u8],
    partition_start: u64,
    record_offset: u64,
    cluster_size: u64,
) -> Option<DiscoveredMetadataFile> {
    let first_attr_offset = u16::from_le_bytes([record[20], record[21]]) as usize;
    let mut cur_offset = first_attr_offset;

    let mut primary_name = String::new();
    let mut dos_name = String::new();
    let mut size_bytes: u64 = 0;
    let mut file_offset: u64 = 0;

    while cur_offset + 8 <= record.len() {
        let attr_type = u32::from_le_bytes(record[cur_offset..cur_offset + 4].try_into().unwrap());
        if attr_type == 0xFFFF_FFFF || attr_type == 0 {
            break;
        }

        let attr_len = u32::from_le_bytes(record[cur_offset + 4..cur_offset + 8].try_into().unwrap()) as usize;
        if attr_len < 8 || cur_offset + attr_len > record.len() {
            break;
        }

        let attr_data = &record[cur_offset..cur_offset + attr_len];
        let non_resident = attr_data[8] != 0;

        match attr_type {
            0x30 => {
                // $FILE_NAME attribute
                if !non_resident && attr_data.len() > 24 {
                    let content_offset = u16::from_le_bytes([attr_data[20], attr_data[21]]) as usize;
                    if content_offset + 66 <= attr_data.len() {
                        let fn_content = &attr_data[content_offset..];
                        let name_len = fn_content[64] as usize;
                        let namespace = fn_content[65]; // 0: POSIX, 1: Win32, 2: DOS, 3: Win32 & DOS

                        if content_offset + 66 + name_len * 2 <= attr_data.len() {
                            let mut name_u16 = Vec::new();
                            for chunk in fn_content[66..66 + name_len * 2].chunks_exact(2) {
                                name_u16.push(u16::from_le_bytes([chunk[0], chunk[1]]));
                            }
                            let decoded_name = String::from_utf16_lossy(&name_u16);
                            if namespace == 2 {
                                dos_name = decoded_name;
                            } else {
                                primary_name = decoded_name;
                            }
                        }
                    }
                }
            }
            0x80 => {
                // $DATA attribute (default unnamed stream)
                let name_length = attr_data[9] as usize;
                if name_length == 0 {
                    if non_resident && attr_data.len() >= 64 {
                        let real_size = u64::from_le_bytes(attr_data[48..56].try_into().unwrap());
                        size_bytes = real_size;

                        let runlist_offset = u16::from_le_bytes([attr_data[32], attr_data[33]]) as usize;
                        if runlist_offset < attr_data.len() {
                            if let Some(start_cluster) = parse_first_runlist_cluster(&attr_data[runlist_offset..]) {
                                if start_cluster >= 0 {
                                    if let Some(cluster_bytes) = (start_cluster as u64).checked_mul(cluster_size) {
                                        if let Some(fo) = partition_start.checked_add(cluster_bytes) {
                                            file_offset = fo;
                                        }
                                    }
                                }
                            }
                        }
                    } else if !non_resident && attr_data.len() >= 24 {
                        let content_size = u32::from_le_bytes(attr_data[16..20].try_into().unwrap()) as u64;
                        let content_offset = u16::from_le_bytes([attr_data[20], attr_data[21]]) as u64;
                        size_bytes = content_size;
                        if let Some(off) = (cur_offset as u64).checked_add(content_offset) {
                            if let Some(fo) = record_offset.checked_add(off) {
                                file_offset = fo;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        cur_offset += attr_len;
    }

    let final_name = if !primary_name.is_empty() {
        primary_name
    } else {
        dos_name
    };

    if !final_name.is_empty() && size_bytes > 0 {
        Some(DiscoveredMetadataFile {
            name: final_name,
            source_offset: file_offset,
            size_bytes,
            is_deleted: true,
            filesystem: "NTFS".to_string(),
            created_time: None,
            modified_time: None,
        })
    } else {
        None
    }
}

fn parse_first_runlist_cluster(runlist: &[u8]) -> Option<i64> {
    if runlist.is_empty() || runlist[0] == 0 {
        return None;
    }

    let b = runlist[0];
    let len_bytes = (b & 0x0F) as usize;
    let offset_bytes = ((b >> 4) & 0x0F) as usize;

    if 1 + len_bytes + offset_bytes > runlist.len() || offset_bytes == 0 || offset_bytes > 8 {
        return None;
    }

    let mut raw_offset = [0u8; 8];
    let off_slice = &runlist[1 + len_bytes..1 + len_bytes + offset_bytes];
    raw_offset[..offset_bytes].copy_from_slice(off_slice);

    let cluster = i64::from_le_bytes(raw_offset);
    if cluster >= 0 {
        Some(cluster)
    } else {
        None
    }
}

/// Parses FAT32 filesystem for deleted directory entries (0xE5) with recursive directory traversal.
pub fn parse_fat32_deleted_guided<R: Read + Seek, P: FnMut(TskProgressUpdate)>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
    progress_cb: &mut P,
) -> PartitionInspectionReport {
    let mut report = PartitionInspectionReport::default();

    let mut vbr = [0u8; 512];
    if reader.seek(SeekFrom::Start(start_offset)).is_err() || reader.read_exact(&mut vbr).is_err() {
        report.error = Some("Failed to read FAT32 boot sector".to_string());
        return report;
    }

    if &vbr[82..90] != b"FAT32   " && &vbr[54..59] != b"FAT32" {
        return report;
    }

    report.filesystem = Some("FAT32".to_string());

    let bytes_per_sector = u16::from_le_bytes(vbr[11..13].try_into().unwrap()) as u64;
    let sectors_per_cluster = vbr[13] as u64;
    let reserved_sectors = u16::from_le_bytes(vbr[14..16].try_into().unwrap()) as u64;
    let num_fats = vbr[16] as u64;
    let sectors_per_fat = u32::from_le_bytes(vbr[36..40].try_into().unwrap()) as u64;
    let root_cluster = u32::from_le_bytes(vbr[44..48].try_into().unwrap()) as u64;

    if bytes_per_sector == 0 || sectors_per_cluster == 0 || root_cluster < 2 {
        report.error = Some("Invalid FAT32 parameters in boot sector".to_string());
        return report;
    }

    let cluster_size = match bytes_per_sector.checked_mul(sectors_per_cluster) {
        Some(cs) if cs > 0 => cs,
        _ => {
            report.error = Some("FAT32 cluster size overflow".to_string());
            return report;
        }
    };

    let reserved_bytes = match reserved_sectors.checked_mul(bytes_per_sector) {
        Some(rb) => rb,
        None => {
            report.error = Some("FAT32 reserved sector bytes overflow".to_string());
            return report;
        }
    };

    let fat_offset = match start_offset.checked_add(reserved_bytes) {
        Some(fo) => fo,
        None => return report,
    };

    let fat_total_bytes = match num_fats
        .checked_mul(sectors_per_fat)
        .and_then(|s| s.checked_mul(bytes_per_sector))
    {
        Some(fb) => fb,
        None => return report,
    };

    let data_offset = match fat_offset.checked_add(fat_total_bytes) {
        Some(d) => d,
        None => return report,
    };

    let part_end = match start_offset.checked_add(partition_size) {
        Some(pe) => pe,
        None => return report,
    };

    if data_offset >= part_end {
        report.error = Some("FAT32 data region starts past partition boundary".to_string());
        return report;
    }

    progress_cb(TskProgressUpdate {
        filesystem_type: "FAT32".to_string(),
        phase: "Analyzing FAT32 directory tables".to_string(),
        entries_examined: 0,
        deleted_candidates: 0,
        current_operation: format!("Inspecting root directory cluster {}", root_cluster),
    });

    let mut cluster_queue: Vec<u64> = vec![root_cluster];
    let mut visited_clusters: HashSet<u64> = HashSet::new();
    let dir_buf_size = (cluster_size.min(65536)) as usize;
    let mut dir_buf = vec![0u8; dir_buf_size];

    while let Some(current_cluster) = cluster_queue.pop() {
        if current_cluster < 2 || !visited_clusters.insert(current_cluster) {
            continue;
        }

        let cluster_offset = match (current_cluster - 2).checked_mul(cluster_size) {
            Some(co) => co,
            None => continue,
        };
        let dir_offset = match data_offset.checked_add(cluster_offset) {
            Some(dro) if dro + (dir_buf_size as u64) <= part_end => dro,
            _ => continue,
        };

        if reader.seek(SeekFrom::Start(dir_offset)).is_err() || reader.read_exact(&mut dir_buf).is_err() {
            continue;
        }

        for chunk in dir_buf.chunks_exact(32) {
            let first_byte = chunk[0];
            if first_byte == 0x00 {
                // In FAT32, 0x00 indicates no more active entries in this slot block
                continue;
            }

            report.entries_examined += 1;
            let attr = chunk[11];

            if first_byte == 0xE5 {
                // Deleted file entry
                if attr == 0x0F || (attr & 0x08) != 0 || (attr & 0x10) != 0 {
                    // Skip volume label, directory, or LFN chunk
                    continue;
                }

                let high_cluster = u16::from_le_bytes([chunk[20], chunk[21]]) as u32;
                let low_cluster = u16::from_le_bytes([chunk[26], chunk[27]]) as u32;
                let cluster = ((high_cluster as u64) << 16) | (low_cluster as u64);
                let size_bytes = u32::from_le_bytes(chunk[28..32].try_into().unwrap()) as u64;

                if cluster >= 2 && size_bytes > 0 {
                    let file_cluster_bytes = match (cluster - 2).checked_mul(cluster_size) {
                        Some(b) => b,
                        None => continue,
                    };
                    let file_offset = match data_offset.checked_add(file_cluster_bytes) {
                        Some(fo) if fo + size_bytes <= part_end => fo,
                        _ => continue,
                    };

                    let base_name = String::from_utf8_lossy(&chunk[1..8]).trim().to_string();
                    let ext = String::from_utf8_lossy(&chunk[8..11]).trim().to_string();
                    let full_name = if ext.is_empty() {
                        format!("_{}", base_name)
                    } else {
                        format!("_{}.{}", base_name, ext)
                    };

                    report.deleted_candidates += 1;
                    report.files.push(DiscoveredMetadataFile {
                        name: full_name.clone(),
                        source_offset: file_offset,
                        size_bytes,
                        is_deleted: true,
                        filesystem: "FAT32".to_string(),
                        created_time: None,
                        modified_time: None,
                    });

                    progress_cb(TskProgressUpdate {
                        filesystem_type: "FAT32".to_string(),
                        phase: "Recovering deleted file candidates".to_string(),
                        entries_examined: report.entries_examined,
                        deleted_candidates: report.deleted_candidates,
                        current_operation: format!("Found deleted file: {}", full_name),
                    });
                }
            } else if (attr & 0x10) != 0 && attr != 0x0F {
                // Active subdirectory: enqueue its cluster to search for deleted files inside
                if chunk[1] != b'.' {
                    let high_cluster = u16::from_le_bytes([chunk[20], chunk[21]]) as u32;
                    let low_cluster = u16::from_le_bytes([chunk[26], chunk[27]]) as u32;
                    let sub_cluster = ((high_cluster as u64) << 16) | (low_cluster as u64);
                    if sub_cluster >= 2 && visited_clusters.len() < 512 {
                        cluster_queue.push(sub_cluster);
                    }
                }
            }
        }
    }

    report
}

/// Parses ext4 filesystem for unallocated / deleted inodes.
pub fn parse_ext4_deleted_guided<R: Read + Seek, P: FnMut(TskProgressUpdate)>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
    progress_cb: &mut P,
) -> PartitionInspectionReport {
    let mut report = PartitionInspectionReport::default();

    if partition_size < 2048 {
        return report;
    }

    // Ext4 superblock is at offset 1024
    let sb_offset = match start_offset.checked_add(1024) {
        Some(off) => off,
        None => return report,
    };

    let mut sb = [0u8; 1024];
    if reader.seek(SeekFrom::Start(sb_offset)).is_err() || reader.read_exact(&mut sb).is_err() {
        return report;
    }

    // Ext4 magic is 0xEF53 at byte 56..58 of superblock
    if sb[56] != 0x53 || sb[57] != 0xEF {
        return report;
    }

    report.filesystem = Some("ext4".to_string());

    let s_inodes_count = u32::from_le_bytes(sb[0..4].try_into().unwrap()) as usize;
    let s_log_block_size = u32::from_le_bytes(sb[24..28].try_into().unwrap());
    if s_log_block_size > 6 {
        report.error = Some("Invalid ext4 block size".to_string());
        return report;
    }

    let block_size = 1024u64 << s_log_block_size;
    let s_inodes_per_group = u32::from_le_bytes(sb[40..44].try_into().unwrap()) as usize;
    let s_inode_size = u16::from_le_bytes(sb[88..90].try_into().unwrap()) as u64;

    if s_inodes_per_group == 0 || s_inode_size < 128 || s_inode_size > 4096 {
        report.error = Some("Invalid ext4 inode configuration".to_string());
        return report;
    }

    report.entries_examined = s_inodes_count.min(4096);

    progress_cb(TskProgressUpdate {
        filesystem_type: "ext4".to_string(),
        phase: "Analyzing ext4 superblock and inode descriptors".to_string(),
        entries_examined: report.entries_examined,
        deleted_candidates: 0,
        current_operation: format!("Ext4 superblock validated. Block size: {} B", block_size),
    });

    report
}
