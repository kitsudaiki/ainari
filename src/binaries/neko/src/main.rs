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

mod command_rules;
mod regex_rules;

use log::LevelFilter;

use tokio::process::Command;
use tonic::{Request, Response, Status, transport::Server};

#[allow(clippy::result_large_err)]
pub mod root_wrapper {
    tonic::include_proto!("root_wrapper");
}

use root_wrapper::neko_root_wrapper_server::{NekoRootWrapper, NekoRootWrapperServer};
use root_wrapper::{CommandRequest, CommandResponse};

use crate::command_rules::COMMAND_RULES;

#[derive(Debug, Default)]
pub struct Checker;

impl Checker {
    fn is_allowed(&self, cmd: &str, args: &[String]) -> bool {
        COMMAND_RULES.iter().any(|rule| rule.matches(cmd, args))
    }
}

#[tonic::async_trait]
impl NekoRootWrapper for Checker {
    async fn execute(
        &self,
        request: Request<CommandRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        let req = request.into_inner();

        if !self.is_allowed(&req.command, &req.args) {
            log::error!(
                "[REJECTED] Unauthorized command pattern: {} {:?}",
                req.command,
                req.args
            );
            return Err(Status::permission_denied(
                "Command or argument pattern not permitted by policy",
            ));
        }

        log::info!(
            "[EXECUTING] Allowed command: {} {:?}",
            req.command,
            req.args
        );

        // Execute via Tokio async Command (bypasses shell expansion)
        match Command::new(&req.command).args(&req.args).output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let exit_code = output.status.code().unwrap_or(-1);

                Ok(Response::new(CommandResponse {
                    success: output.status.success(),
                    stdout,
                    stderr,
                    exit_code,
                    error_message: String::new(),
                }))
            }
            Err(e) => Ok(Response::new(CommandResponse {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: -1,
                error_message: format!("Command execution failed: {}", e),
            })),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::set_max_level(LevelFilter::Info);

    let addr = "127.0.0.1:54515".parse()?;
    let daemon = Checker;

    log::info!("Root wrapper daemon active on {}", addr);

    Server::builder()
        .add_service(NekoRootWrapperServer::new(daemon))
        .serve(addr)
        .await?;

    Ok(())
}
