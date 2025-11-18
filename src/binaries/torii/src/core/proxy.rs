// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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

pub struct Proxy {
    #[allow(dead_code)]
    pub handle: Option<tokio::task::JoinHandle<()>>,

    #[allow(dead_code)]
    pub public_addr: SocketAddr,
    #[allow(dead_code)]
    pub sakura_addr: String,

    pub shutdown_tx: watch::Sender<()>,
}

impl Proxy {
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

    pub fn stop(&mut self) {
        log::debug!("Stop proxy with listen-address '{}'", self.public_addr);
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}

fn remove_http_prefix(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
}

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
