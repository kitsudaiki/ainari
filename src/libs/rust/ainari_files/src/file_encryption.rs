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

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};

use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

/// Length of the nonce (initialization vector) used for encryption
/// Recommended length for GCM mode is 12 bytes
const NONCE_LEN: usize = 12;
/// Length of the authentication tag used for GCM mode
const TAG_LEN: usize = 16;
/// Size of the chunk used for reading/writing files (1 MiB)
const CHUNK_SIZE: usize = 1024 * 1024;
/// Size of the encryption key in bytes (256 bits)
const KEY_SIZE: usize = 32;

/// Decodes a base64-encoded key into its binary representation
///
/// # Arguments
///
/// * `key_b64` - A secret containing the base64-encoded key
///
/// # Returns
///
/// * `Result<Vec<u8>, AinariError>` - The decoded key bytes or an error
///
/// # Errors
///
/// Returns `AinariError::InvalidInput` if:
/// * The input is not a valid base64 string
/// * The decoded key length is not equal to `KEY_SIZE`
fn decode_base64_key(key_b64: &Secret) -> Result<Vec<u8>, AinariError> {
    // decode base64 key
    let key_bytes = match STANDARD.decode(key_b64.reveal().as_bytes()) {
        Ok(key_bytes) => key_bytes,
        Err(_) => {
            // HINT (kitsudaiki): do NOT use the error-message of the decode-function to avoid the risk
            // of printing information of the key in the log-output
            let msg = "Provided key is not a valid base64-encoded string.".to_string();
            return Err(AinariError::InvalidInput(msg));
        }
    };
    if key_bytes.len() != KEY_SIZE {
        let msg = format!(
            "Invalid key length: expected {}, got {}",
            KEY_SIZE,
            key_bytes.len()
        );
        return Err(AinariError::InvalidInput(msg));
    }

    Ok(key_bytes)
}

/// Encrypts a file using AES-256-GCM encryption
///
/// # Arguments
///
/// * `in_path` - Path to the input file to be encrypted
/// * `out_path` - Path where the encrypted file will be saved
/// * `key_b64` - A secret containing the base64-encoded encryption key
///
/// # Returns
///
/// * `Result<(), AinariError>` - Success or an error
///
/// # Errors
///
/// Returns `AinariError::InvalidInput` if:
/// * The key is invalid
/// * The input file is too short
///
/// Returns `AinariError::InternalError` for filesystem or encryption-related errors
pub async fn encrypt_file(
    in_path: &String,
    out_path: &String,
    key_b64: &Secret,
) -> Result<(), AinariError> {
    // Decode the base64 key and verify its length
    let key_bytes = decode_base64_key(key_b64)?;
    let key: &[u8] = &key_bytes;

    if key.len() != 32 {
        return Err(AinariError::InvalidInput(
            "Key must be 32 bytes (256 bits)".to_string(),
        ));
    }

    // Generate a random nonce (initialization vector) for encryption
    let mut nonce = [0u8; NONCE_LEN];
    rand_bytes(&mut nonce).map_err(|e| {
        AinariError::InternalError(format!("Error while generating random nonce: {e}"))
    })?;

    // Create the AES-256-GCM cipher with the provided key and nonce
    let cipher = Cipher::aes_256_gcm();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(&nonce))
        .map_err(|e| AinariError::InternalError(format!("Error while creating crypter: {e}")))?;

    // Optional: Add Authentication Data (AAD) if needed
    // crypter.aad_update(b"optional AAD")?;

    // Open the input file for reading
    let infile = File::open(in_path)
        .map_err(|e| AinariError::InternalError(format!("Error while opening input file: {e}")))?;
    let mut reader = BufReader::new(infile);

    // Create the output file for writing the encrypted data
    let outfile = File::create(out_path).map_err(|e| {
        AinariError::InternalError(format!("Error while creating output file: {e}"))
    })?;
    let mut writer = BufWriter::new(outfile);

    // Write the nonce to the output file first (needed for decryption)
    writer.write_all(&nonce).map_err(|e| {
        AinariError::InternalError(format!("Error while writing encrypted data to disk: {e}"))
    })?;

    // Buffers for reading/writing data in chunks
    let mut in_buf = vec![0u8; CHUNK_SIZE];
    // OpenSSL may produce up to (in_len + block_size - 1) bytes; allocate extra space
    let mut out_buf = vec![0u8; CHUNK_SIZE + cipher.block_size()];

    // Read the input file in chunks, encrypt each chunk, and write to the output file
    loop {
        let n = reader.read(&mut in_buf).map_err(|e| {
            AinariError::InternalError(format!(
                "Error while reading data for encryption from disk: {e}"
            ))
        })?;
        if n == 0 {
            break;
        }
        let count = crypter
            .update(&in_buf[..n], &mut out_buf)
            .map_err(|e| AinariError::InternalError(format!("Error while encrypting data: {e}")))?;
        writer.write_all(&out_buf[..count]).map_err(|e| {
            AinariError::InternalError(format!("Error while writing encrypted data to disk: {e}"))
        })?;
    }

    // Finalize the encryption and write any remaining bytes
    let count = crypter
        .finalize(&mut out_buf)
        .map_err(|e| AinariError::InternalError(format!("Error while encrypting data: {e}")))?;
    if count > 0 {
        writer.write_all(&out_buf[..count]).map_err(|e| {
            AinariError::InternalError(format!("Error while writing encrypted data to disk: {e}"))
        })?;
    }

    // Get the GCM authentication tag and append it to the encrypted data
    let mut tag = vec![0u8; TAG_LEN];
    crypter
        .get_tag(&mut tag)
        .map_err(|e| AinariError::InternalError(format!("Error while encrypting data: {e}")))?;
    writer.write_all(&tag).map_err(|e| {
        AinariError::InternalError(format!("Error while writing encrypted data to disk: {e}"))
    })?;

    // Ensure all data is written to disk
    writer.flush().map_err(|e| {
        AinariError::InternalError(format!("Error while flushing encrypted data to disk: {e}"))
    })?;
    Ok(())
}

/// Decrypts a file encrypted with AES-256-GCM
///
/// # Arguments
///
/// * `in_path` - Path to the encrypted file
/// * `out_path` - Path where the decrypted file will be saved
/// * `key_b64` - A secret containing the base64-encoded decryption key
///
/// # Returns
///
/// * `Result<(), AinariError>` - Success or an error
///
/// # Errors
///
/// Returns `AinariError::InvalidInput` if:
/// * The key is invalid
/// * The input file is too short
/// * The authentication tag is invalid (indicating tampering or wrong key)
///
/// Returns `AinariError::InternalError` for filesystem or decryption-related errors
pub async fn decrypt_file(
    in_path: &String,
    out_path: &String,
    key_b64: &Secret,
) -> Result<(), AinariError> {
    // Decode the base64 key and verify its length
    let key_bytes = decode_base64_key(key_b64)?;
    let key: &[u8] = &key_bytes;

    if key.len() != 32 {
        return Err(AinariError::InvalidInput(
            "Key must be 32 bytes (256 bits)".to_string(),
        ));
    }

    // Open the encrypted file for reading
    let infile = File::open(in_path).map_err(|e| {
        AinariError::InternalError(format!("Error while opening file for decryption: {e}"))
    })?;

    // Get the file size to determine the positions of the nonce and tag
    let metadata = std::fs::metadata(in_path).map_err(|e| {
        AinariError::InternalError(format!(
            "Error while reading file metadata for decryption: {e}"
        ))
    })?;
    let total_len = metadata.len() as usize;

    // Verify the file is long enough to contain nonce and tag
    if total_len < NONCE_LEN + TAG_LEN {
        return Err(AinariError::InvalidInput(
            "Input file too short".to_string(),
        ));
    }

    // Read the nonce from the beginning of the file
    let mut reader = BufReader::new(infile);
    let mut nonce = [0u8; NONCE_LEN];
    reader.read_exact(&mut nonce).map_err(|e| {
        AinariError::InternalError(format!("Error while reading encrypted data from disk: {e}"))
    })?;

    // Calculate the length of the ciphertext (total length minus nonce and tag)
    let ciphertext_len = total_len - NONCE_LEN - TAG_LEN;

    // Read the authentication tag from the end of the file
    let mut infile_for_tag = File::open(in_path).map_err(|e| {
        AinariError::InternalError(format!("Error while opening file for decryption: {e}"))
    })?;
    infile_for_tag
        .seek(SeekFrom::Start((total_len - TAG_LEN) as u64))
        .map_err(|e| AinariError::InternalError(format!("Error while seeking in file: {e}")))?;
    let mut tag = [0u8; TAG_LEN];
    infile_for_tag.read_exact(&mut tag).map_err(|e| {
        AinariError::InternalError(format!("Error while reading encrypted data from disk: {e}"))
    })?;

    // Create the AES-256-GCM cipher with the provided key and nonce
    let cipher = Cipher::aes_256_gcm();
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(&nonce))
        .map_err(|e| AinariError::InternalError(format!("Error while decrypting data: {e}")))?;
    crypter
        .set_tag(&tag)
        .map_err(|e| AinariError::InternalError(format!("Error while decrypting data: {e}")))?;

    // Create a temporary file for writing the decrypted data
    let tmp_path = format!("{out_path}tmp");
    let tmp_file = File::create(&tmp_path).map_err(|e| {
        AinariError::InternalError(format!(
            "Error while creating temp file for decryption: {e}"
        ))
    })?;
    let mut writer = BufWriter::new(tmp_file);

    // Track remaining bytes to decrypt
    let mut remaining = ciphertext_len;
    let mut in_buf = vec![0u8; CHUNK_SIZE];
    let mut out_buf = vec![0u8; CHUNK_SIZE + cipher.block_size()];

    // Read the ciphertext in chunks, decrypt each chunk, and write to the temporary file
    while remaining > 0 {
        let to_read = std::cmp::min(remaining, CHUNK_SIZE);
        let n = reader.read(&mut in_buf[..to_read]).map_err(|e| {
            AinariError::InternalError(format!("Error while reading encrypted data from disk: {e}"))
        })?;
        if n == 0 {
            break;
        }
        remaining -= n;

        let count = crypter
            .update(&in_buf[..n], &mut out_buf)
            .map_err(|_| AinariError::InternalError("Failed to decrypt file".to_string()))?;
        writer.write_all(&out_buf[..count]).map_err(|e| {
            AinariError::InternalError(format!("Error while writing decrypted data to disk: {e}"))
        })?;
    }

    // Verify the authentication tag; if wrong, finalize will return an error
    if crypter.finalize(&mut out_buf).is_err() {
        // Remove the temporary file if decryption fails
        let _ = fs::remove_file(&tmp_path);
        return Err(AinariError::InvalidInput(
            "Invalid key or tampered file".to_string(),
        ));
    }

    // Flush any remaining bytes to disk
    writer.flush().map_err(|e| {
        AinariError::InternalError(format!("Error while flushing decrypted data to disk: {e}"))
    })?;

    // Rename the temporary file to the final output path
    std::fs::rename(tmp_path, out_path).map_err(|e| {
        AinariError::InternalError(format!("Error while writing decrypted data to disk: {e}"))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::rand::rand_bytes;
    use std::fs::{File, remove_file};
    use std::io::{Read, Write};

    fn create_random_file(path: &String, size: usize) {
        let mut f = File::create(path).expect("create random file");
        let mut remaining = size;
        let mut buf = vec![0u8; CHUNK_SIZE];

        while remaining > 0 {
            let to_write = std::cmp::min(remaining, CHUNK_SIZE);
            rand_bytes(&mut buf[..to_write]).expect("rand_bytes");
            f.write_all(&buf[..to_write]).expect("write random");
            remaining -= to_write;
        }
        f.flush().expect("flush random file");
    }

    fn read_all(path: &String) -> Vec<u8> {
        let mut v = Vec::new();
        let mut f = File::open(path).expect("open file for read");
        f.read_to_end(&mut v).expect("read_to_end");
        v
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn encrypt_decrypt_roundtrip_10mb() {
        let file_size = 10 * 1024 * 1024;
        let in_path = "/tmp/test_10mb_input.bin".to_owned();
        let enc_path = "/tmp/test_10mb_encrypted.bin".to_owned();
        let dec_path = "/tmp/test_10mb_decrypted.bin".to_owned();
        let key_b64 = Secret::from("q9vN4CjOQm5wKzyzjZtS7t4oQp8oQK1JvU5xgq8vFzE=");
        let other_key_b64 = Secret::from("Wv7gD9jR5mM8zX4oU1kTb2eYvG0qHp9wF3sLrNdChKI=");
        let invalid_key_b64 = Secret::from("oQK1JvU5xgq8vFzE=");

        // Ensure no leftover files
        let _ = remove_file(&in_path);
        let _ = remove_file(&enc_path);
        let _ = remove_file(&dec_path);

        // create input
        create_random_file(&in_path, file_size);

        // test encrypt and decrypt
        encrypt_file(&in_path, &enc_path, &key_b64).await.unwrap();
        decrypt_file(&enc_path, &dec_path, &key_b64).await.unwrap();

        // read original and decrypted, compare full contents
        let orig = read_all(&in_path);
        let encrypted = read_all(&enc_path);
        let recovered = read_all(&dec_path);
        assert_eq!(orig.len(), recovered.len(), "length mismatch");
        assert_ne!(orig, encrypted, "encrypted content does match original");
        assert_eq!(orig, recovered, "decrypted content does not match original");

        // test with invalid key 1
        let ret_dec1 = decrypt_file(&enc_path, &dec_path, &other_key_b64).await;
        assert!(ret_dec1.is_err());

        // test with invalid key 2
        let ret_dec2 = decrypt_file(&enc_path, &dec_path, &invalid_key_b64).await;
        assert!(ret_dec2.is_err());

        // cleanup
        let _ = remove_file(&in_path);
        let _ = remove_file(&enc_path);
        let _ = remove_file(&dec_path);
    }
}
