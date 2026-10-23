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

//! Reads the server name (SNI) out of the TLS ClientHello at the start of a connection.
//!
//! The proxies don't terminate TLS, so they only look at the first bytes of a connection, which
//! are the plain ClientHello, and forward these bytes unchanged afterwards.

/// Upper limit of the bytes, which are read for a ClientHello. A ClientHello is usually far
/// below 1 KiB, but post-quantum key-shares make it larger, so it can span several records.
pub const MAX_CLIENT_HELLO_SIZE: usize = 16 * 1024;

/// Content-type of a TLS-record with handshake-messages
const RECORD_TYPE_HANDSHAKE: u8 = 22;
/// Size of the header of a TLS-record: type (1), version (2), length (2)
const RECORD_HEADER_SIZE: usize = 5;
/// Type of the handshake-message ClientHello
const HANDSHAKE_TYPE_CLIENT_HELLO: u8 = 1;
/// Size of the header of a handshake-message: type (1), length (3)
const HANDSHAKE_HEADER_SIZE: usize = 4;
/// Type of the extension server_name (RFC 6066)
const EXTENSION_SERVER_NAME: u16 = 0;
/// Type of an entry of the server_name-extension, which contains a DNS host name
const NAME_TYPE_HOST_NAME: u8 = 0;

/// Result of reading the server name out of the start of a connection
#[derive(Debug, PartialEq, Eq)]
pub enum SniResult {
    /// The ClientHello is not complete yet, more bytes have to be read
    Incomplete,
    /// The server name of the ClientHello
    Found(String),
    /// The connection doesn't start with a valid TLS ClientHello or it has no server name
    Invalid(&'static str),
}

/// Reads the server name out of the start of a connection.
///
/// The ClientHello can be split over several TLS-records, so the handshake-message is collected
/// from all records first.
///
/// # Arguments
/// * `data` - All bytes, which were read from the connection so far
///
/// # Returns
/// The server name, `Incomplete` if more bytes are required or `Invalid` if there is no server
/// name to find
pub fn read_server_name(data: &[u8]) -> SniResult {
    let mut handshake = Vec::new();
    let mut rest = data;

    loop {
        if rest.len() < RECORD_HEADER_SIZE {
            return SniResult::Incomplete;
        }
        if rest[0] != RECORD_TYPE_HANDSHAKE {
            return SniResult::Invalid("not a TLS handshake");
        }
        let record_len = u16::from_be_bytes([rest[3], rest[4]]) as usize;
        if record_len == 0 {
            return SniResult::Invalid("empty TLS record");
        }
        let Some(fragment) = rest.get(RECORD_HEADER_SIZE..RECORD_HEADER_SIZE + record_len) else {
            return SniResult::Incomplete;
        };
        handshake.extend_from_slice(fragment);
        rest = &rest[RECORD_HEADER_SIZE + record_len..];

        if handshake.len() < HANDSHAKE_HEADER_SIZE {
            continue;
        }
        if handshake[0] != HANDSHAKE_TYPE_CLIENT_HELLO {
            return SniResult::Invalid("not a TLS ClientHello");
        }
        let message_len =
            u32::from_be_bytes([0, handshake[1], handshake[2], handshake[3]]) as usize;
        if message_len > MAX_CLIENT_HELLO_SIZE {
            return SniResult::Invalid("TLS ClientHello is too large");
        }
        if let Some(body) =
            handshake.get(HANDSHAKE_HEADER_SIZE..HANDSHAKE_HEADER_SIZE + message_len)
        {
            return match server_name_of_client_hello(body) {
                Some(Ok(name)) => SniResult::Found(name),
                Some(Err(reason)) => SniResult::Invalid(reason),
                None => SniResult::Invalid("malformed TLS ClientHello"),
            };
        }
    }
}

/// Simple reader over the fields of a handshake-message. Every getter returns `None`, if the
/// message is shorter than the field.
struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.data.len() < len {
            return None;
        }
        let (head, tail) = self.data.split_at(len);
        self.data = tail;
        Some(head)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.bytes(2)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a block, which is prefixed with its length in one byte
    fn block_u8(&mut self) -> Option<Reader<'a>> {
        let len = self.u8()? as usize;
        Some(Reader {
            data: self.bytes(len)?,
        })
    }

    /// Reads a block, which is prefixed with its length in two bytes
    fn block_u16(&mut self) -> Option<Reader<'a>> {
        let len = self.u16()? as usize;
        Some(Reader {
            data: self.bytes(len)?,
        })
    }
}

/// Reads the server name out of the body of a ClientHello.
///
/// # Arguments
/// * `body` - The ClientHello without its handshake-header
///
/// # Returns
/// `None` if the message is malformed, otherwise the server name or the reason, why there is no
/// usable one
fn server_name_of_client_hello(body: &[u8]) -> Option<Result<String, &'static str>> {
    let mut hello = Reader { data: body };
    // legacy_version and random
    hello.bytes(2 + 32)?;
    // legacy_session_id, cipher_suites and legacy_compression_methods
    hello.block_u8()?;
    hello.block_u16()?;
    hello.block_u8()?;
    if hello.data.is_empty() {
        return Some(Err("TLS ClientHello without extensions"));
    }

    let mut extensions = hello.block_u16()?;
    while !extensions.data.is_empty() {
        let extension_type = extensions.u16()?;
        let mut extension = extensions.block_u16()?;
        if extension_type != EXTENSION_SERVER_NAME {
            continue;
        }

        let mut names = extension.block_u16()?;
        while !names.data.is_empty() {
            let name_type = names.u8()?;
            let name = names.block_u16()?.data;
            if name_type != NAME_TYPE_HOST_NAME {
                continue;
            }
            return Some(match std::str::from_utf8(name) {
                Ok(name) if !name.is_empty() && name.is_ascii() => Ok(name.to_owned()),
                _ => Err("invalid server name in TLS ClientHello"),
            });
        }
    }

    Some(Err("TLS ClientHello without server name"))
}

/// Builders of ClientHellos for the tests
#[cfg(test)]
pub mod test_util {
    use super::*;

    /// Builds a block with its length in two bytes in front of it
    pub fn block_u16(content: &[u8]) -> Vec<u8> {
        let mut block = (content.len() as u16).to_be_bytes().to_vec();
        block.extend_from_slice(content);
        block
    }

    /// Builds the server_name-extension with the given name
    pub fn server_name_extension(name: &str) -> Vec<u8> {
        let mut entry = vec![NAME_TYPE_HOST_NAME];
        entry.extend(block_u16(name.as_bytes()));
        let mut extension = EXTENSION_SERVER_NAME.to_be_bytes().to_vec();
        extension.extend(block_u16(&block_u16(&entry)));
        extension
    }

    /// Builds a ClientHello-handshake-message with the given extensions. Another extension in
    /// front of them makes sure, that the extensions are really iterated.
    pub fn client_hello(extensions: Option<&[u8]>) -> Vec<u8> {
        let mut body = vec![0x03, 0x03];
        body.extend([7u8; 32]);
        // session id
        body.push(32);
        body.extend([1u8; 32]);
        // cipher suites
        body.extend(block_u16(&[0x13, 0x01, 0x13, 0x02]));
        // compression methods
        body.extend([1, 0]);
        if let Some(extensions) = extensions {
            // supported_versions
            let mut all = vec![0x00, 0x2b];
            all.extend(block_u16(&[0x02, 0x03, 0x04]));
            all.extend_from_slice(extensions);
            body.extend(block_u16(&all));
        }

        let mut message = vec![HANDSHAKE_TYPE_CLIENT_HELLO];
        message.extend(&(body.len() as u32).to_be_bytes()[1..]);
        message.extend(body);
        message
    }

    /// Wraps a handshake-message into TLS-records with at most `chunk` bytes each
    pub fn records(message: &[u8], chunk: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for fragment in message.chunks(chunk) {
            data.extend([RECORD_TYPE_HANDSHAKE, 0x03, 0x01]);
            data.extend(block_u16(fragment));
        }
        data
    }

    /// Builds the TLS-records of a ClientHello with the given server name
    pub fn client_hello_with_name(name: &str) -> Vec<u8> {
        records(&client_hello(Some(&server_name_extension(name))), 16 * 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;

    #[test]
    fn the_server_name_is_found() {
        let hello = client_hello(Some(&server_name_extension("abc.ainari.localhost")));
        let data = records(&hello, 16 * 1024);
        assert_eq!(
            read_server_name(&data),
            SniResult::Found("abc.ainari.localhost".to_owned())
        );

        // bytes after the ClientHello, like the next records, don't matter
        let mut more = data.clone();
        more.extend([23, 3, 3, 0, 1, 0]);
        assert_eq!(
            read_server_name(&more),
            SniResult::Found("abc.ainari.localhost".to_owned())
        );
    }

    #[test]
    fn a_client_hello_over_several_records_is_collected() {
        let hello = client_hello(Some(&server_name_extension("abc.ainari.localhost")));
        let data = records(&hello, 3);
        assert_eq!(
            read_server_name(&data),
            SniResult::Found("abc.ainari.localhost".to_owned())
        );
    }

    #[test]
    fn an_incomplete_client_hello_requires_more_bytes() {
        let hello = client_hello(Some(&server_name_extension("abc.ainari.localhost")));
        let data = records(&hello, 20);
        for len in 0..data.len() {
            assert_eq!(
                read_server_name(&data[..len]),
                SniResult::Incomplete,
                "{len}"
            );
        }
    }

    #[test]
    fn a_connection_without_server_name_is_invalid() {
        let without_extensions = records(&client_hello(None), 16 * 1024);
        assert!(matches!(
            read_server_name(&without_extensions),
            SniResult::Invalid(_)
        ));

        let without_name = records(&client_hello(Some(&[])), 16 * 1024);
        assert!(matches!(
            read_server_name(&without_name),
            SniResult::Invalid(_)
        ));
    }

    #[test]
    fn other_protocols_are_invalid() {
        assert!(matches!(
            read_server_name(b"GET / HTTP/1.1\r\nHost: abc\r\n\r\n"),
            SniResult::Invalid(_)
        ));

        // a handshake-record, which is not a ClientHello
        let mut server_hello = client_hello(None);
        server_hello[0] = 2;
        assert!(matches!(
            read_server_name(&records(&server_hello, 16 * 1024)),
            SniResult::Invalid(_)
        ));
    }

    #[test]
    fn a_malformed_client_hello_is_invalid() {
        let mut hello = client_hello(Some(&server_name_extension("abc.ainari.localhost")));
        // cut the message, but keep the length in its header
        let len = hello.len();
        hello.truncate(len - 4);
        let body_len = (hello.len() - HANDSHAKE_HEADER_SIZE) as u32;
        hello[1..4].copy_from_slice(&body_len.to_be_bytes()[1..]);
        assert!(matches!(
            read_server_name(&records(&hello, 16 * 1024)),
            SniResult::Invalid(_)
        ));
    }

    #[test]
    fn a_too_large_client_hello_is_invalid() {
        let mut hello = client_hello(None);
        hello[1..4].copy_from_slice(&((MAX_CLIENT_HELLO_SIZE + 1) as u32).to_be_bytes()[1..]);
        assert!(matches!(
            read_server_name(&records(&hello[..HANDSHAKE_HEADER_SIZE], 16 * 1024)),
            SniResult::Invalid(_)
        ));
    }
}
