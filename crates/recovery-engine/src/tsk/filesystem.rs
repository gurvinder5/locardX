use serde::{Deserialize, Serialize};
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

/// Inspects a partition for deleted files via filesystem metadata structures.
pub fn inspect_partition_filesystem<R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
    filesystem_hint: Option<&str>,
) -> Vec<DiscoveredMetadataFile> {
    let mut discovered = Vec::new();

    let fs_type = filesystem_hint
        .map(|s| s.to_uppercase())
        .unwrap_or_default();

    if fs_type.contains("NTFS") {
        let ntfs_files = parse_ntfs_deleted(reader, start_offset, partition_size);
        discovered.extend(ntfs_files);
    } else if fs_type.contains("FAT32") {
        let fat_files = parse_fat32_deleted(reader, start_offset, partition_size);
        discovered.extend(fat_files);
    } else {
        // Try NTFS then FAT32 if hint is unknown
        let ntfs_files = parse_ntfs_deleted(reader, start_offset, partition_size);
        if !ntfs_files.is_empty() {
            discovered.extend(ntfs_files);
        } else {
            let fat_files = parse_fat32_deleted(reader, start_offset, partition_size);
            discovered.extend(fat_files);
        }
    }

    discovered
}

/// Parses NTFS filesystem for deleted MFT records.
fn parse_ntfs_deleted<R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
) -> Vec<DiscoveredMetadataFile> {
    let mut files = Vec::new();

    let mut vbr = [0u8; 512];
    if reader.seek(SeekFrom::Start(start_offset)).is_err() {
        return files;
    }
    if reader.read_exact(&mut vbr).is_err() {
        return files;
    }

    if &vbr[3..11] != b"NTFS    " {
        return files;
    }

    let bytes_per_sector = u16::from_le_bytes(vbr[11..13].try_into().unwrap()) as u64;
    let sectors_per_cluster = vbr[13] as u64;
    if bytes_per_sector == 0 || sectors_per_cluster == 0 {
        return files;
    }

    let cluster_size = bytes_per_sector * sectors_per_cluster;
    let mft_lcn = i64::from_le_bytes(vbr[48..56].try_into().unwrap());
    if mft_lcn < 0 {
        return files;
    }

    let mft_record_size: u64 = {
        let val = vbr[64] as i8;
        if val < 0 {
            1u64 << (-val as u32)
        } else {
            (val as u64) * cluster_size
        }
    };

    if mft_record_size < 512 || mft_record_size > 4096 {
        return files;
    }

    let mft_start = start_offset + (mft_lcn as u64) * cluster_size;
    if mft_start >= start_offset + partition_size {
        return files;
    }

    // Inspect first 256 MFT records safely
    let records_to_scan = 256usize;
    let mut record_buf = vec![0u8; mft_record_size as usize];

    for idx in 0..records_to_scan {
        let record_offset = mft_start + (idx as u64) * mft_record_size;
        if record_offset + mft_record_size > start_offset + partition_size {
            break;
        }

        if reader.seek(SeekFrom::Start(record_offset)).is_err() {
            break;
        }
        if reader.read_exact(&mut record_buf).is_err() {
            break;
        }

        if &record_buf[0..4] != b"FILE" {
            continue;
        }

        let flags = u16::from_le_bytes([record_buf[22], record_buf[23]]);
        let is_in_use = (flags & 0x0001) != 0;
        let is_dir = (flags & 0x0002) != 0;

        // We seek deleted files: !is_in_use && !is_dir
        if !is_in_use && !is_dir {
            if let Some(entry) =
                extract_ntfs_mft_file(&record_buf, start_offset, record_offset, cluster_size)
            {
                if entry.size_bytes > 0 && entry.source_offset > 0 {
                    files.push(entry);
                }
            }
        }
    }

    files
}

fn extract_ntfs_mft_file(
    record: &[u8],
    partition_start: u64,
    record_offset: u64,
    cluster_size: u64,
) -> Option<DiscoveredMetadataFile> {
    let first_attr_offset = u16::from_le_bytes([record[20], record[21]]) as usize;
    let mut cur_offset = first_attr_offset;

    let mut name = String::new();
    let mut size_bytes: u64 = 0;
    let mut file_offset: u64 = 0;

    while cur_offset + 8 <= record.len() {
        let attr_type = u32::from_le_bytes(record[cur_offset..cur_offset + 4].try_into().unwrap());
        if attr_type == 0xFFFF_FFFF || attr_type == 0 {
            break;
        }

        let attr_len =
            u32::from_le_bytes(record[cur_offset + 4..cur_offset + 8].try_into().unwrap()) as usize;
        if attr_len < 8 || cur_offset + attr_len > record.len() {
            break;
        }

        let attr_data = &record[cur_offset..cur_offset + attr_len];
        let non_resident = attr_data[8] != 0;

        match attr_type {
            0x30 => {
                //
                if !non_resident && attr_data.len() > 24 {
                    let content_offset =
                        u16::from_le_bytes([attr_data[20], attr_data[21]]) as usize;
                    if content_offset + 66 <= attr_data.len() {
                        let fn_content = &attr_data[content_offset..];
                        let name_len = fn_content[64] as usize;
                        if content_offset + 66 + name_len * 2 <= attr_data.len() {
                            let mut name_u16 = Vec::new();
                            for chunk in fn_content[66..66 + name_len * 2].chunks_exact(2) {
                                name_u16.push(u16::from_le_bytes([chunk[0], chunk[1]]));
                            }
                            name = String::from_utf16_lossy(&name_u16);
                        }
                    }
                }
            }
            0x80 => {
                //
                if non_resident && attr_data.len() >= 64 {
                    let real_size = u64::from_le_bytes(attr_data[48..56].try_into().unwrap());
                    size_bytes = real_size;

                    let runlist_offset =
                        u16::from_le_bytes([attr_data[32], attr_data[33]]) as usize;
                    if runlist_offset < attr_data.len() {
                        if let Some(start_cluster) =
                            parse_first_runlist_cluster(&attr_data[runlist_offset..])
                        {
                            file_offset = partition_start + (start_cluster as u64) * cluster_size;
                        }
                    }
                } else if !non_resident && attr_data.len() >= 24 {
                    let content_size =
                        u32::from_le_bytes(attr_data[16..20].try_into().unwrap()) as u64;
                    let content_offset = u16::from_le_bytes([attr_data[20], attr_data[21]]) as u64;
                    size_bytes = content_size;
                    file_offset = record_offset + (cur_offset as u64) + content_offset;
                }
            }
            _ => {}
        }

        cur_offset += attr_len;
    }

    if !name.is_empty() && size_bytes > 0 {
        Some(DiscoveredMetadataFile {
            name,
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

    // Sign extension if highest bit is set
    if off_slice.last().copied().unwrap_or(0) & 0x80 != 0 {
        for byte in &mut raw_offset[offset_bytes..] {
            *byte = 0xFF;
        }
    }

    Some(i64::from_le_bytes(raw_offset))
}

/// Parses FAT32 filesystem for deleted directory entries (marked with 0xE5).
fn parse_fat32_deleted<R: Read + Seek>(
    reader: &mut R,
    start_offset: u64,
    partition_size: u64,
) -> Vec<DiscoveredMetadataFile> {
    let mut files = Vec::new();

    let mut vbr = [0u8; 512];
    if reader.seek(SeekFrom::Start(start_offset)).is_err() {
        return files;
    }
    if reader.read_exact(&mut vbr).is_err() {
        return files;
    }

    if &vbr[82..90] != b"FAT32   " && &vbr[54..59] != b"FAT32" {
        return files;
    }

    let bytes_per_sector = u16::from_le_bytes(vbr[11..13].try_into().unwrap()) as u64;
    let sectors_per_cluster = vbr[13] as u64;
    let reserved_sectors = u16::from_le_bytes(vbr[14..16].try_into().unwrap()) as u64;
    let num_fats = vbr[16] as u64;
    let sectors_per_fat = u32::from_le_bytes(vbr[36..40].try_into().unwrap()) as u64;
    let root_cluster = u32::from_le_bytes(vbr[44..48].try_into().unwrap()) as u64;

    if bytes_per_sector == 0 || sectors_per_cluster == 0 || root_cluster < 2 {
        return files;
    }

    let cluster_size = bytes_per_sector * sectors_per_cluster;
    let fat_offset = start_offset + reserved_sectors * bytes_per_sector;
    let data_offset = fat_offset + num_fats * sectors_per_fat * bytes_per_sector;

    if data_offset >= start_offset + partition_size {
        return files;
    }

    // Inspect root directory cluster (and up to 8 subsequent clusters)
    let root_dir_offset = data_offset + (root_cluster - 2) * cluster_size;
    let mut dir_buf = vec![0u8; (cluster_size.min(65536)) as usize];

    if reader.seek(SeekFrom::Start(root_dir_offset)).is_ok()
        && reader.read_exact(&mut dir_buf).is_ok()
    {
        for chunk in dir_buf.chunks_exact(32) {
            let first_byte = chunk[0];
            if first_byte == 0x00 {
                break; // End of directory
            }

            if first_byte == 0xE5 {
                // Deleted entry!
                let attr = chunk[11];
                if attr == 0x0F || (attr & 0x10) != 0 {
                    continue; // Skip LFN and directory
                }

                let high_cluster = u16::from_le_bytes([chunk[20], chunk[21]]) as u32;
                let low_cluster = u16::from_le_bytes([chunk[26], chunk[27]]) as u32;
                let cluster = (high_cluster << 16) | low_cluster;
                let size_bytes = u32::from_le_bytes(chunk[28..32].try_into().unwrap()) as u64;

                if cluster >= 2 && size_bytes > 0 {
                    let file_offset = data_offset + ((cluster as u64) - 2) * cluster_size;
                    let base_name = String::from_utf8_lossy(&chunk[1..8]).trim().to_string();
                    let ext = String::from_utf8_lossy(&chunk[8..11]).trim().to_string();
                    let full_name = if ext.is_empty() {
                        format!("_{}", base_name)
                    } else {
                        format!("_{}.{}", base_name, ext)
                    };

                    files.push(DiscoveredMetadataFile {
                        name: full_name,
                        source_offset: file_offset,
                        size_bytes,
                        is_deleted: true,
                        filesystem: "FAT32".to_string(),
                        created_time: None,
                        modified_time: None,
                    });
                }
            }
        }
    }

    files
}
