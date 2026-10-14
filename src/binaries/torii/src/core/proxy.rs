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

use tokio::sync::watch;

use anyhow::{Result, anyhow};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::config::ProxyConf;
use crate::core::proxy_handler::PROXY_HANDLER;
use crate::core::sni::{MAX_CLIENT_HELLO_SIZE, SniResult, read_server_name};

/// Time, which a client has to send its TLS ClientHello after it connected, so idle connections
/// don't pile up in front of the proxies
const CLIENT_HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// Target of a proxy, which the connections with its server name are forwarded to.
///
/// Dropping it closes all connections, which are open over the proxy, so a proxy, which is
/// deleted or pointed at another sakura-host, doesn't keep the old connections.
pub struct Proxy {
    /// Address of the sakura-host, which the connections are forwarded to
    pub target_addr: String,
    /// Closes the open connections of the proxy, when it is dropped
    shutdown_tx: watch::Sender<()>,
}

impl Proxy {
    /// Creates a new proxy, which forwards to the given target.
    ///
    /// # Arguments
    ///
    /// * `target_addr` - The target address to forward connections to.
    ///
    /// # Returns
    ///
    /// The new proxy.
    pub fn new(target_addr: &str) -> Self {
        let (shutdown_tx, _) = watch::channel(());
        Proxy {
            target_addr: remove_http_prefix(target_addr).to_owned(),
            shutdown_tx,
        }
    }

    /// Returns a receiver, which is notified, when the open connections of the proxy have to be
    /// closed.
    pub fn shutdown_signal(&self) -> watch::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Closes all open connections of the proxy.
    ///
    /// This method is idempotent and can be called multiple times safely.
    pub fn stop(&mut self) {
        log::debug!("Stop proxy to '{}'", self.target_addr);
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for Proxy {
    /// Ensures the connections of the proxy are closed when the Proxy is dropped.
    fn drop(&mut self) {
        self.stop();
    }
}

/// Removes the HTTP/HTTPS prefix from a URL if present.
///
/// # Arguments
///
/// * `url` - The URL to process.
///
/// # Returns
///
/// A string slice with the prefix removed if it existed, or the original URL otherwise.
fn remove_http_prefix(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
}

/// Binds the listener of all proxies and accepts its connections in the background.
///
/// # Arguments
///
/// * `conf` - The configuration of the proxies.
/// * `listen_addr` - The local address to bind to.
///
/// # Returns
///
/// `Ok(())` if the listener runs, or an error if the address could not be bound.
pub async fn start_listener(conf: &'static ProxyConf, listen_addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(listen_addr).await?;
    log::info!(
        "Proxies listen on {listen_addr} for '<proxy-uuid>.{}'",
        conf.domain
    );

    tokio::spawn(async move {
        loop {
            let (inbound, peer) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    log::error!("Accept failed: {e}");
                    continue;
                }
            };

            tokio::spawn(async move {
                if let Err(e) = handle_one_connection(conf, inbound).await {
                    log::debug!("Proxy-connection from {peer} closed: {e}");
                }
            });
        }
    });

    Ok(())
}

/// Reads the TLS ClientHello at the start of a connection.
///
/// # Arguments
///
/// * `inbound` - The incoming TCP stream from the client.
///
/// # Returns
///
/// The server name of the ClientHello together with all bytes, which were read so far, so they
/// can be forwarded to the target unchanged.
async fn read_client_hello(inbound: &mut TcpStream) -> Result<(String, Vec<u8>)> {
    let mut data = Vec::with_capacity(1024);
    loop {
        // a few bytes on top for the headers of the records, over which the message is split
        if data.len() > MAX_CLIENT_HELLO_SIZE + 1024 {
            return Err(anyhow!("TLS ClientHello is too large"));
        }
        if inbound.read_buf(&mut data).await? == 0 {
            return Err(anyhow!("connection closed before the TLS ClientHello"));
        }
        match read_server_name(&data) {
            SniResult::Incomplete => continue,
            SniResult::Found(server_name) => return Ok((server_name, data)),
            SniResult::Invalid(reason) => return Err(anyhow!(reason)),
        }
    }
}

/// Handles a single TCP connection by picking the proxy of its server name and forwarding the
/// data bidirectionally between the client and the target of the proxy.
///
/// # Arguments
///
/// * `conf` - The configuration of the proxies.
/// * `inbound` - The incoming TCP stream from the client.
///
/// # Returns
///
/// `Ok(())` if the connection is closed gracefully, or an error if something went wrong.
async fn handle_one_connection(conf: &ProxyConf, mut inbound: TcpStream) -> Result<()> {
    let (server_name, client_hello) =
        tokio::time::timeout(CLIENT_HELLO_TIMEOUT, read_client_hello(&mut inbound))
            .await
            .map_err(|_| anyhow!("no TLS ClientHello within {CLIENT_HELLO_TIMEOUT:?}"))??;

    let proxy_uuid = conf
        .proxy_of_server_name(&server_name)
        .ok_or_else(|| anyhow!("server name '{server_name}' is not the one of a proxy"))?;
    let (target_addr, mut shutdown) = {
        let proxy_handler = PROXY_HANDLER.read().await;
        let proxy = proxy_handler
            .proxys
            .get(&proxy_uuid)
            .ok_or_else(|| anyhow!("proxy '{proxy_uuid}' doesn't exist"))?;
        (proxy.target_addr.clone(), proxy.shutdown_signal())
    };

    let mut outbound = TcpStream::connect(&target_addr).await?;
    outbound.write_all(&client_hello).await?;

    tokio::select! {
        res = tokio::io::copy_bidirectional(&mut inbound, &mut outbound) => {
            let (n1, n2) = res?;
            log::debug!(
                "Transferred {} bytes (in->out) and {} bytes (out->in) for {}",
                n1 + client_hello.len() as u64, n2, target_addr
            );
        }
        _ = shutdown.changed() => {
            log::debug!("Proxy '{proxy_uuid}' was changed, closing its connection");
            let _ = inbound.shutdown().await;
            let _ = outbound.shutdown().await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sni::test_util::client_hello_with_name;
    use serial_test::serial;
    use uuid::Uuid;

    /// Upper limit for every step of the tests, so a broken proxy fails instead of hanging
    const STEP_TIMEOUT: Duration = Duration::from_secs(5);

    /// Starts the listener of the proxies on a free port of localhost
    async fn start_test_listener() -> SocketAddr {
        let conf: &'static ProxyConf = Box::leak(Box::new(ProxyConf {
            domain: "ainari.localhost".to_owned(),
            port: 0,
            public_port: None,
            listen_ip: None,
        }));
        let addr = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap();
        start_listener(conf, addr).await.unwrap();
        addr
    }

    /// Registers a new proxy, which forwards to a new target on localhost
    async fn add_test_proxy() -> (Uuid, TcpListener) {
        let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let uuid = Uuid::new_v4();
        let target_addr = format!("http://{}", target.local_addr().unwrap());
        PROXY_HANDLER
            .write()
            .await
            .add_proxy(&uuid, &target_addr)
            .unwrap();
        (uuid, target)
    }

    /// Reads, until the connection is closed, and returns the number of bytes read
    async fn read_until_closed(stream: &mut TcpStream) -> usize {
        let mut data = Vec::new();
        let _ = tokio::time::timeout(STEP_TIMEOUT, stream.read_to_end(&mut data))
            .await
            .expect("connection was not closed");
        data.len()
    }

    #[tokio::test]
    #[serial]
    async fn connections_are_forwarded_by_their_server_name() {
        let listen_addr = start_test_listener().await;
        let (uuid, target) = add_test_proxy().await;
        let (_other_uuid, _other_target) = add_test_proxy().await;

        // the name is case-insensitive, like DNS
        let hello = client_hello_with_name(&format!("{uuid}.Ainari.Localhost"));
        let mut client = TcpStream::connect(listen_addr).await.unwrap();
        client.write_all(&hello).await.unwrap();
        client.write_all(b"request").await.unwrap();

        // the target gets the ClientHello and everything after it unchanged
        let (mut server, _) = tokio::time::timeout(STEP_TIMEOUT, target.accept())
            .await
            .unwrap()
            .unwrap();
        let mut received = vec![0u8; hello.len() + b"request".len()];
        tokio::time::timeout(STEP_TIMEOUT, server.read_exact(&mut received))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&received[..hello.len()], &hello[..]);
        assert_eq!(&received[hello.len()..], b"request");

        server.write_all(b"response").await.unwrap();
        let mut answer = [0u8; 8];
        tokio::time::timeout(STEP_TIMEOUT, client.read_exact(&mut answer))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&answer, b"response");

        PROXY_HANDLER.write().await.delete_proxy(&uuid).unwrap();
        // the deleted proxy closes its open connection
        assert_eq!(read_until_closed(&mut client).await, 0);
    }

    #[tokio::test]
    #[serial]
    async fn connections_without_a_known_proxy_are_closed() {
        let listen_addr = start_test_listener().await;

        for data in [
            client_hello_with_name(&format!("{}.ainari.localhost", Uuid::new_v4())),
            client_hello_with_name("ainari.localhost"),
            b"GET / HTTP/1.1\r\nHost: ainari.localhost\r\n\r\n".to_vec(),
        ] {
            let mut client = TcpStream::connect(listen_addr).await.unwrap();
            client.write_all(&data).await.unwrap();
            assert_eq!(read_until_closed(&mut client).await, 0);
        }
    }

    #[tokio::test]
    #[serial]
    async fn a_retargeted_proxy_closes_its_connections() {
        let listen_addr = start_test_listener().await;
        let (uuid, target) = add_test_proxy().await;

        let hello = client_hello_with_name(&format!("{uuid}.ainari.localhost"));
        let mut client = TcpStream::connect(listen_addr).await.unwrap();
        client.write_all(&hello).await.unwrap();
        let (_server, _) = tokio::time::timeout(STEP_TIMEOUT, target.accept())
            .await
            .unwrap()
            .unwrap();

        let new_target = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let new_target_addr = new_target.local_addr().unwrap().to_string();
        PROXY_HANDLER
            .write()
            .await
            .retarget_proxy(&uuid, &new_target_addr)
            .unwrap();
        assert_eq!(read_until_closed(&mut client).await, 0);

        // new connections reach the new target under the same name
        let mut client = TcpStream::connect(listen_addr).await.unwrap();
        client.write_all(&hello).await.unwrap();
        tokio::time::timeout(STEP_TIMEOUT, new_target.accept())
            .await
            .unwrap()
            .unwrap();

        PROXY_HANDLER.write().await.delete_proxy(&uuid).unwrap();
    }
}
