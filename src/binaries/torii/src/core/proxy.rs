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

use anyhow::Result;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

/// A TCP proxy that forwards connections from a public address to a target address.
///
/// This struct manages the proxy lifecycle, including starting and stopping the proxy server.
/// It uses a watch channel for graceful shutdown signaling.
pub struct Proxy {
    /// Handle to the async task running the proxy server.
    #[allow(dead_code)]
    pub handle: Option<tokio::task::JoinHandle<()>>,

    /// The public address that the proxy listens on.
    #[allow(dead_code)]
    pub public_addr: SocketAddr,

    /// The target address that connections will be forwarded to.
    #[allow(dead_code)]
    pub sakura_addr: String,

    /// Sender half of the watch channel for shutdown signaling.
    pub shutdown_tx: watch::Sender<()>,
}

impl Proxy {
    /// Creates a new Proxy instance and starts listening on the specified address.
    ///
    /// # Arguments
    ///
    /// * `public_addr` - The local address to bind the proxy server to.
    /// * `sakura_addr` - The target address to forward connections to.
    ///
    /// # Returns
    ///
    /// A new Proxy instance with the server running in the background.
    pub async fn new(public_addr: &SocketAddr, sakura_addr: &str) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        let public_addr_clone = *public_addr;
        let sakura_addr_clone = sakura_addr.to_owned();

        let handle = tokio::spawn(async move {
            if let Err(e) = run_listener(public_addr_clone, sakura_addr_clone, shutdown_rx).await {
                eprintln!("Listener {} -> failed: {e}", public_addr_clone);
            }
        });

        Proxy {
            handle: Some(handle),
            public_addr: *public_addr,
            sakura_addr: sakura_addr.to_owned(),
            shutdown_tx,
        }
    }

    /// Stops the proxy server by sending a shutdown signal.
    ///
    /// This method is idempotent and can be called multiple times safely.
    pub fn stop(&mut self) {
        log::debug!("Stop proxy with listen-address '{}'", self.public_addr);
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for Proxy {
    /// Ensures the proxy is stopped when the Proxy instance is dropped.
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
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

/// Runs the main listener loop for the proxy server.
///
/// This function binds to the specified address and accepts incoming connections,
/// forwarding them to the target address.
///
/// # Arguments
///
/// * `listen_addr` - The local address to bind to.
/// * `target_addr` - The target address to forward connections to.
/// * `shutdown` - Receiver half of the watch channel for shutdown signaling.
///
/// # Returns
///
/// `Ok(())` if the listener is stopped gracefully, or an error if something went wrong.
async fn run_listener(
    listen_addr: SocketAddr,
    target_addr: String,
    mut shutdown: watch::Receiver<()>,
) -> Result<()> {
    let listener = TcpListener::bind(listen_addr).await?;
    log::debug!("Listening on {} -> {}", listen_addr, target_addr);

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                log::debug!("Run-listener: shutdown received, stopping accept loop");
                break Ok(()); // drop listener and stop accepting
            }
            accept_res = listener.accept() => {
                let (mut inbound, peer) = match accept_res {
                    Ok(pair) => pair,
                    Err(e) => {
                        log::error!("Accept failed: {}", e);
                        continue;
                    }
                };

                // prepare per-connection state
                let target = remove_http_prefix(&target_addr).to_owned();
                let mut conn_shutdown = shutdown.clone(); // clone for the spawned task

                // move inbound (owned) into the spawned task — avoids borrow issues
                tokio::spawn(async move {
                    if let Err(e) = handle_one_connection(&mut inbound, &target, &mut conn_shutdown).await {
                        log::error!("Connection {} -> {} error: {}", peer, target, e);
                    }
                });
            }
        }
    }
}

/// Handles a single TCP connection by forwarding data bidirectionally between the client and target.
///
/// # Arguments
///
/// * `inbound` - The incoming TCP stream from the client.
/// * `target_addr` - The target address to connect to.
/// * `shutdown` - Receiver half of the watch channel for shutdown signaling.
///
/// # Returns
///
/// `Ok(())` if the connection is closed gracefully, or an error if something went wrong.
async fn handle_one_connection(
    inbound: &mut TcpStream,
    target_addr: &str,
    shutdown: &mut watch::Receiver<()>,
) -> Result<()> {
    let mut outbound = TcpStream::connect(target_addr).await?;

    tokio::select! {
        res = tokio::io::copy_bidirectional(inbound, &mut outbound) => {
            let (n1, n2) = res.unwrap();
            log::debug!(
                "Transferred {} bytes (in->out) and {} bytes (out->in) for {}",
                n1, n2, target_addr
            );
        }
        _ = shutdown.changed() => {
            log::debug!("Shutdown signal received, closing connection");
            let _ = inbound.shutdown().await;
            let _ = outbound.shutdown().await;
        }
    }

    Ok(())
}
