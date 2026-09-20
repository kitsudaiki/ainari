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

pub async fn init_neko_root_wrapper_client() -> Result<NekoRootWrapperClient<Channel>, AinariError>
{
    NekoRootWrapperClient::connect("http://127.0.0.1:54515")
        .await
        .map_err(|e| {
            AinariError::InternalError(format!("Connection to Neko-root-wrapper failed: {}", e))
        })
}

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
