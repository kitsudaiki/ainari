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

use tonic::transport::Channel;

pub mod root_wrapper {
    tonic::include_proto!("root_wrapper");
}

use root_wrapper::CommandRequest;
use root_wrapper::neko_root_wrapper_client::NekoRootWrapperClient;

use ainari_common::error::*;

/// Opens a connection to the neko-root-wrapper.
///
/// The neko runs as separate daemon with root-privileges and is only reachable on the loopback
/// interface, so the services themselves do not have to run privileged in order to execute the few
/// commands, which need it.
///
/// # Returns
///
/// * `Ok(NekoRootWrapperClient)` - The connected client.
/// * `Err(AinariError::InternalError)` - The neko could not be reached.
pub async fn init_neko_root_wrapper_client() -> Result<NekoRootWrapperClient<Channel>, AinariError>
{
    NekoRootWrapperClient::connect("http://127.0.0.1:54515")
        .await
        .map_err(|e| {
            AinariError::InternalError(format!("Connection to Neko-root-wrapper failed: {}", e))
        })
}

/// Runs a single command with root-privileges over the neko-root-wrapper.
///
/// The command and its arguments are checked against the allow-list of the neko before they are
/// executed, so only the known-good commands can be run this way. The command is executed without
/// a shell, so the arguments are not expanded.
///
/// # Arguments
///
/// * `client` - Connected client of the neko-root-wrapper
/// * `cmd` - Command to execute
/// * `args` - Arguments of the command
///
/// # Returns
///
/// * `Ok(())` - The command was executed and returned successfully.
/// * `Err(AinariError::InternalError)` - The neko was not reachable, rejected the command or the
///   command itself failed. The message contains stderr of the command, if it has written
///   something, else its exit-code.
pub async fn run_root_cmd(
    client: &mut NekoRootWrapperClient<Channel>,
    cmd: &str,
    args: &[&str],
) -> Result<(), AinariError> {
    let request = tonic::Request::new(CommandRequest {
        command: cmd.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
    });

    let response = client
        .execute(request)
        .await
        .map_err(|e| AinariError::InternalError(format!("gRPC Error from Neko: {}", e.message())))?
        .into_inner();

    if response.success {
        Ok(())
    } else {
        let err_msg = if !response.stderr.is_empty() {
            response.stderr.trim().to_string()
        } else {
            format!("exit code {}", response.exit_code)
        };
        Err(AinariError::InternalError(err_msg))
    }
}
