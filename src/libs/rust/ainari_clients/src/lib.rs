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

pub mod auth;
pub mod checkpoint;
pub mod dataset;

use awc::{Client, Connector};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use std::time::Duration;

pub fn prepare_client(use_ssl: bool, insecure: bool) -> Client {
    if use_ssl {
        let mut ssl_builder = SslConnector::builder(SslMethod::tls()).unwrap();

        if insecure {
            ssl_builder.set_verify(SslVerifyMode::NONE);
            ssl_builder.set_verify_callback(SslVerifyMode::NONE, |_, _| true);
        }

        let connector = Connector::new().openssl(ssl_builder.build());
        Client::builder()
            .connector(connector) // pass connector directly
            .timeout(Duration::from_secs(60))
            .finish()
    } else {
        Client::new()
    }
}
