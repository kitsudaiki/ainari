// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use ainari_common::error::AinariError;

/// Size of the sector, which contains the footer of a vhd-image
const VHD_FOOTER_SIZE: u64 = 512;
/// Offset of the primary volume descriptor of an iso-image
const ISO_MAGIC_OFFSET: usize = 0x8001;
/// Offset of the signature of a vdi-image
const VDI_MAGIC_OFFSET: usize = 0x40;
/// Number of bytes at the beginning of a file, which are enough to identify all supported types
const HEADER_SIZE: usize = ISO_MAGIC_OFFSET + 5;

/// Information about a file, which are measured before the file is encrypted
#[derive(Debug, Clone, PartialEq)]
pub struct FileInfo {
    /// Type of the data within the file, named like the formats of `qemu-img` (e.g. "qcow2")
    pub file_type: String,
    /// Size of the file in bytes
    pub file_size: u64,
}

/// Identifies the type of a disk-image by its magic bytes and measures its size
///
/// The type is named like the formats of `qemu-img`. A file, which matches none of the known
/// formats, is handled as `raw`, because a raw disk-image has no header to identify it.
///
/// # Arguments
///
/// * `path` - Path to the unencrypted file
///
/// # Returns
///
/// * `Result<FileInfo, AinariError>` - The type and size of the file or an error
///
/// # Errors
///
/// Returns `AinariError::InternalError`, if the file can not be read
pub async fn get_disk_image_info(path: &str) -> Result<FileInfo, AinariError> {
    let mut file = File::open(path)
        .map_err(|e| AinariError::internal(format!("Error while opening file '{path}': {e}")))?;
    let file_size = file
        .metadata()
        .map_err(|e| AinariError::internal(format!("Error while reading size of '{path}': {e}")))?
        .len();

    // read the beginning of the file, which is shorter than the header for small files
    let mut header = Vec::with_capacity(HEADER_SIZE);
    file.by_ref()
        .take(HEADER_SIZE as u64)
        .read_to_end(&mut header)
        .map_err(|e| AinariError::internal(format!("Error while reading file '{path}': {e}")))?;

    // a fixed vhd-image has its only header as footer at the end of the file
    let mut footer = Vec::new();
    if file_size >= VHD_FOOTER_SIZE {
        file.seek(SeekFrom::Start(file_size - VHD_FOOTER_SIZE))
            .and_then(|_| file.read_to_end(&mut footer))
            .map_err(|e| {
                AinariError::internal(format!("Error while reading file '{path}': {e}"))
            })?;
    }

    Ok(FileInfo {
        file_type: detect_disk_image_type(&header, &footer).to_string(),
        file_size,
    })
}

/// Identifies the type of a disk-image by its magic bytes
///
/// # Arguments
///
/// * `header` - First bytes of the file
/// * `footer` - Last 512 bytes of the file, or empty, if the file is smaller
///
/// # Returns
///
/// The name of the type, like it is used by `qemu-img`
fn detect_disk_image_type(header: &[u8], footer: &[u8]) -> &'static str {
    let has_magic =
        |offset: usize, magic: &[u8]| header.get(offset..offset + magic.len()) == Some(magic);

    if has_magic(0, b"QFI\xfb") {
        // qcow and qcow2 share the magic and are distinguished by the big-endian version
        return match header.get(4..8) {
            Some([0, 0, 0, 1]) => "qcow",
            _ => "qcow2",
        };
    }
    if has_magic(0, b"QED\0") {
        return "qed";
    }
    if has_magic(0, b"vhdxfile") {
        return "vhdx";
    }
    if has_magic(0, b"KDMV") || has_magic(0, b"COWD") {
        return "vmdk";
    }
    if has_magic(VDI_MAGIC_OFFSET, &[0x7f, 0x10, 0xda, 0xbe]) {
        return "vdi";
    }
    // a dynamic vhd-image has a copy of the footer at the beginning
    if has_magic(0, b"conectix") || footer.starts_with(b"conectix") {
        return "vpc";
    }
    if has_magic(ISO_MAGIC_OFFSET, b"CD001") {
        return "iso";
    }

    "raw"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::remove_file;
    use std::io::Write;

    fn header_with_magic(offset: usize, magic: &[u8]) -> Vec<u8> {
        let mut header = vec![0u8; HEADER_SIZE];
        header[offset..offset + magic.len()].copy_from_slice(magic);
        header
    }

    #[test]
    fn detect_known_types() {
        let mut qcow2 = header_with_magic(0, b"QFI\xfb");
        qcow2[7] = 3;
        assert_eq!(detect_disk_image_type(&qcow2, &[]), "qcow2");
        let mut qcow = header_with_magic(0, b"QFI\xfb");
        qcow[7] = 1;
        assert_eq!(detect_disk_image_type(&qcow, &[]), "qcow");
        assert_eq!(
            detect_disk_image_type(&header_with_magic(0, b"QED\0"), &[]),
            "qed"
        );
        assert_eq!(
            detect_disk_image_type(&header_with_magic(0, b"vhdxfile"), &[]),
            "vhdx"
        );
        assert_eq!(
            detect_disk_image_type(&header_with_magic(0, b"KDMV"), &[]),
            "vmdk"
        );
        assert_eq!(
            detect_disk_image_type(
                &header_with_magic(VDI_MAGIC_OFFSET, &[0x7f, 0x10, 0xda, 0xbe]),
                &[]
            ),
            "vdi"
        );
        assert_eq!(
            detect_disk_image_type(&header_with_magic(0, b"conectix"), &[]),
            "vpc"
        );
        assert_eq!(
            detect_disk_image_type(&header_with_magic(ISO_MAGIC_OFFSET, b"CD001"), &[]),
            "iso"
        );
    }

    #[test]
    fn detect_fixed_vhd_by_footer() {
        let mut footer = vec![0u8; VHD_FOOTER_SIZE as usize];
        footer[..8].copy_from_slice(b"conectix");
        assert_eq!(detect_disk_image_type(&[0u8; 64], &footer), "vpc");
    }

    #[test]
    fn detect_raw_for_unknown_and_short_data() {
        assert_eq!(detect_disk_image_type(&[0u8; HEADER_SIZE], &[]), "raw");
        assert_eq!(detect_disk_image_type(b"QF", &[]), "raw");
        assert_eq!(detect_disk_image_type(&[], &[]), "raw");
    }

    #[tokio::test]
    async fn disk_image_info_of_file() {
        let path = "/tmp/test_disk_image_info.qcow2".to_owned();
        let mut content = header_with_magic(0, b"QFI\xfb");
        content[7] = 3;
        content.resize(100_000, 0);
        File::create(&path).unwrap().write_all(&content).unwrap();

        let info = get_disk_image_info(&path).await.unwrap();
        assert_eq!(info.file_type, "qcow2");
        assert_eq!(info.file_size, 100_000);

        remove_file(&path).unwrap();
        assert!(get_disk_image_info(&path).await.is_err());
    }
}
