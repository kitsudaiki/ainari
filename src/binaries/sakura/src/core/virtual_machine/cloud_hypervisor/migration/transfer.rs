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

//! Transfer of the files of a virtual_machine from the source to the target host.
//!
//! The files are sent as a single zstd-frame. The root-disk is a sparse raw-file, whose unused
//! parts are zeros, which compress to almost nothing, so no temporary copy is required on the
//! source. The frame carries the size and a checksum of the content, so the target detects a
//! damaged or truncated transfer. The target skips blocks of zeros, so the root-disk stays sparse
//! on the target as well.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures::SinkExt;
use futures::channel::mpsc;
use futures::executor::block_on;
use uuid::Uuid;
use zstd::stream::raw::Decoder as RawDecoder;
use zstd::stream::write::Encoder;
use zstd::stream::zio::Writer as ZioWriter;

use ainari_api_structs::migration_structs::MigrationFile;
use ainari_clients::virtual_machine_migration::download_migration_file;
use ainari_common::error::AinariError;

use crate::config;

/// Compression-level of the transfer. The disk-content is mostly zeros or already compressed
/// data, so a higher level costs much cpu-time for little gain.
const COMPRESSION_LEVEL: i32 = 1;

/// Size of the chunks, in which a file is read on the source
const READ_CHUNK_SIZE: usize = 1024 * 1024;

/// Number of compressed chunks, which are buffered between the compression and the response
const STREAM_BUFFER: usize = 8;

/// Size of the blocks, which are checked for zeros on the target, so they are skipped and stay
/// holes in the file. Equal to the block-size of the common filesystems.
const SPARSE_BLOCK_SIZE: u64 = 4096;

/// Suffix of a file, while it is received, so an incomplete file never has the final name
const PARTIAL_SUFFIX: &str = ".part";

/// Opens a file of a virtual_machine and streams it zstd-compressed.
///
/// The compression runs in its own thread, because reading and compressing a disk blocks. It
/// stops, as soon as the receiver of the stream is gone, for example because the target closed
/// the connection.
///
/// # Arguments
/// * `path` - Path of the file
///
/// # Returns
/// * `Ok((u64, Stream))` with the size of the uncompressed file and the stream of its
///   compressed content
/// * `Err(AinariError)` if the file can not be opened
pub fn compressed_file_stream(
    path: &Path,
) -> Result<(u64, mpsc::Receiver<io::Result<Bytes>>), AinariError> {
    let file = File::open(path).map_err(|e| {
        AinariError::InternalError(format!("Failed to open {}: {e}", path.display()))
    })?;
    let size = file
        .metadata()
        .map_err(|e| {
            AinariError::InternalError(format!("Failed to read size of {}: {e}", path.display()))
        })?
        .len();

    let (sender, receiver) = mpsc::channel(STREAM_BUFFER);
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let mut sender = sender;
        if let Err(e) = compress(file, size, &mut ChannelWriter(&mut sender)) {
            // a closed channel means, that the receiver is gone, which has nothing to receive
            // the error anymore
            log::warn!(
                "Compression of {} for its migration stopped: {e}",
                path.display()
            );
            let _ = block_on(sender.send(Err(e)));
        }
    });

    Ok((size, receiver))
}

/// Compresses a file into a single zstd-frame, which contains the size and a checksum of the
/// content.
///
/// # Arguments
/// * `file` - The file to compress
/// * `size` - Size of the file in bytes
/// * `output` - Receives the compressed frame
///
/// # Returns
/// * `Ok(())` if the whole file was compressed
/// * `Err(io::Error)` if the file could not be read or the output not be written
fn compress<W: Write>(mut file: File, size: u64, output: W) -> io::Result<()> {
    let mut encoder = Encoder::new(output, COMPRESSION_LEVEL)?;
    encoder.include_checksum(true)?;
    encoder.include_contentsize(true)?;
    encoder.set_pledged_src_size(Some(size))?;

    let mut buffer = vec![0u8; READ_CHUNK_SIZE];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        encoder.write_all(&buffer[..read])?;
    }

    encoder.finish()?.flush()
}

/// Hands everything, which is written into it, as chunks over to a channel
struct ChannelWriter<'a>(&'a mut mpsc::Sender<io::Result<Bytes>>);

impl Write for ChannelWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        block_on(self.0.send(Ok(Bytes::copy_from_slice(buf))))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "receiver is gone"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Pulls a file of a prepared virtual_machine from the source host.
///
/// The file is written next to its final path first and only moved there, after it was received
/// completely and intact, so an interrupted transfer never leaves a file behind, which looks
/// complete.
///
/// # Arguments
/// * `source_address` - Internal address of the source host
/// * `token` - Token, which authorizes the request on the source host
/// * `uuid` - Unique identifier of the virtual_machine
/// * `file` - The file to transfer
/// * `target_path` - Final path of the file on this host
///
/// # Returns
/// * `Ok(())` if the file was received completely and intact
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn receive_file(
    source_address: &str,
    token: &str,
    uuid: &Uuid,
    file: MigrationFile,
    target_path: &str,
) -> Result<(), AinariError> {
    let partial_path = PathBuf::from(format!("{target_path}{PARTIAL_SUFFIX}"));

    let result = async {
        let sparse_file = SparseFile::create(&partial_path).map_err(|e| {
            AinariError::InternalError(format!("Failed to create {}: {e}", partial_path.display()))
        })?;
        let decoder = RawDecoder::new().map_err(|e| {
            AinariError::InternalError(format!("Failed to create zstd-decoder: {e}"))
        })?;
        let mut writer = ZioWriter::new(sparse_file, decoder);

        let expected_size = download_migration_file(
            source_address,
            token,
            &config::INTERNAL_API_KEY,
            uuid,
            file,
            &mut writer,
            config::CONFIG.skip_tls_verification,
        )
        .await?;

        let received_size = finish_receive(writer)
            .map_err(|e| AinariError::InternalError(format!("Received {file} is damaged: {e}")))?;
        if received_size != expected_size {
            return Err(AinariError::InternalError(format!(
                "Received {file} has {received_size} bytes instead of {expected_size}"
            )));
        }

        fs::rename(&partial_path, target_path).map_err(|e| {
            AinariError::InternalError(format!(
                "Failed to move {} to {target_path}: {e}",
                partial_path.display()
            ))
        })
    }
    .await;

    if result.is_err() {
        let _ = fs::remove_file(&partial_path);
    }
    result
}

/// Ends the decompression of a received file and writes it to the disk.
///
/// # Arguments
/// * `writer` - The decoder, which wrote the received file
///
/// # Returns
/// * `Ok(u64)` with the size of the decompressed file
/// * `Err(io::Error)` if the frame is incomplete or damaged, or the file can not be written
fn finish_receive(mut writer: ZioWriter<SparseFile, RawDecoder<'static>>) -> io::Result<u64> {
    // fails with an incomplete frame, if the transfer broke off at a frame-boundary
    writer.finish()?;
    let (sparse_file, _) = writer.into_inner();
    sparse_file.finish()
}

/// File, which skips the blocks of zeros, which are written into it, so they stay holes and
/// don't take any space on the disk.
///
/// The file is created empty, so the skipped blocks read as zeros afterwards.
struct SparseFile {
    file: File,
    /// Position in the file, where the next write starts
    position: u64,
}

impl SparseFile {
    /// Creates a new, empty file and replaces an existing one.
    fn create(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Ok(Self { file, position: 0 })
    }

    /// Gives the file its full size, also if it ends with skipped zeros, and writes it to the
    /// disk.
    ///
    /// # Returns
    /// * `Ok(u64)` with the size of the file
    fn finish(self) -> io::Result<u64> {
        self.file.set_len(self.position)?;
        self.file.sync_all()?;
        Ok(self.position)
    }

    /// Writes data, which starts at `offset` within the current write-call
    fn write_data(&self, data: &[u8], offset: usize) -> io::Result<()> {
        self.file.write_all_at(data, self.position + offset as u64)
    }
}

impl Write for SparseFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // start of the data in `buf`, which is not written yet, while blocks with data follow
        // each other, so they are written with a single call
        let mut pending_data: Option<usize> = None;
        let mut offset = 0;
        while offset < buf.len() {
            // the chunks follow the blocks of the file, so a block of zeros becomes a hole
            let position = self.position + offset as u64;
            let to_block_end = SPARSE_BLOCK_SIZE - position % SPARSE_BLOCK_SIZE;
            let length = (buf.len() - offset).min(to_block_end as usize);
            let is_zero = buf[offset..offset + length].iter().all(|byte| *byte == 0);

            match (is_zero, pending_data) {
                (false, None) => pending_data = Some(offset),
                (true, Some(start)) => {
                    self.write_data(&buf[start..offset], start)?;
                    pending_data = None;
                }
                _ => {}
            }
            offset += length;
        }
        if let Some(start) = pending_data {
            self.write_data(&buf[start..], start)?;
        }

        self.position += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::os::unix::fs::MetadataExt;

    /// Creates an empty temporary directory, which is removed by the test itself
    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sakura_transfer_{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Content of a disk with data at its start and its middle and zeros at its end
    fn disk_content() -> Vec<u8> {
        let mut content = vec![0u8; 8 * 1024 * 1024 + 123];
        content[..5000].fill(0xab);
        content[4 * 1024 * 1024..4 * 1024 * 1024 + 7].copy_from_slice(b"ainari!");
        content
    }

    /// Compresses a file like the source and returns the frame
    fn compress_file(path: &Path) -> Vec<u8> {
        let (size, stream) = compressed_file_stream(path).unwrap();
        assert_eq!(size, fs::metadata(path).unwrap().len());
        let chunks: Vec<Bytes> = block_on(stream.map(|chunk| chunk.unwrap()).collect());
        chunks.concat()
    }

    /// Writes a frame into a file like the target and returns the size of the file
    fn decompress_into(frame: &[u8], path: &Path) -> io::Result<u64> {
        let mut writer = ZioWriter::new(SparseFile::create(path)?, RawDecoder::new()?);
        // the frame arrives in chunks of any size
        for chunk in frame.chunks(1000) {
            writer.write_all(chunk)?;
        }
        finish_receive(writer)
    }

    #[test]
    fn test_round_trip_keeps_content_and_holes() {
        let dir = temp_dir();
        let source = dir.join("source");
        let target = dir.join("target");
        let content = disk_content();
        fs::write(&source, &content).unwrap();

        let frame = compress_file(&source);
        // the zeros compress to almost nothing
        assert!(frame.len() < content.len() / 100);

        assert_eq!(
            decompress_into(&frame, &target).unwrap(),
            content.len() as u64
        );
        assert_eq!(fs::read(&target).unwrap(), content);

        // the blocks of zeros were skipped, so the file uses less space than its size
        let allocated = fs::metadata(&target).unwrap().blocks() * 512;
        assert!(allocated < content.len() as u64 / 2);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_sparse_file_with_unaligned_writes() {
        let dir = temp_dir();
        let path = dir.join("target");

        // data, which starts and ends within blocks and spans several blocks, between zeros
        let mut content = vec![0u8; 64 * 1024 + 5];
        content[100..200].fill(1);
        content[4090..4100].fill(2);
        content[20_000..30_000].fill(3);
        *content.last_mut().unwrap() = 4;

        for write_size in [1, 777, 4096, 5000, content.len()] {
            let mut sparse_file = SparseFile::create(&path).unwrap();
            for chunk in content.chunks(write_size) {
                sparse_file.write_all(chunk).unwrap();
            }
            assert_eq!(sparse_file.finish().unwrap(), content.len() as u64);
            assert_eq!(fs::read(&path).unwrap(), content, "write-size {write_size}");
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_empty_file() {
        let dir = temp_dir();
        let source = dir.join("source");
        let target = dir.join("target");
        fs::write(&source, b"").unwrap();

        let frame = compress_file(&source);
        assert_eq!(decompress_into(&frame, &target).unwrap(), 0);
        assert!(fs::read(&target).unwrap().is_empty());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_truncated_frame_is_detected() {
        let dir = temp_dir();
        let source = dir.join("source");
        fs::write(&source, disk_content()).unwrap();
        let frame = compress_file(&source);

        for length in [frame.len() - 1, frame.len() / 2, 10] {
            let result = decompress_into(&frame[..length], &dir.join("target"));
            assert!(result.is_err(), "frame cut to {length} bytes was accepted");
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_damaged_frame_is_detected() {
        let dir = temp_dir();
        let source = dir.join("source");
        fs::write(&source, disk_content()).unwrap();
        let mut frame = compress_file(&source);

        // flip a bit in the middle of the frame, which changes the content
        let middle = frame.len() / 2;
        frame[middle] ^= 0x01;
        assert!(decompress_into(&frame, &dir.join("target")).is_err());

        fs::remove_dir_all(&dir).unwrap();
    }
}
