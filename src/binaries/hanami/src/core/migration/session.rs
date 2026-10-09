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

//! Token of a migration, which is renewed at miko, as long as the migration takes.

use std::time::{Duration, Instant};

use crate::config;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::auth::renew_token;

/// Share of the lifetime of a token, after which it is renewed. The rest is the margin for the
/// requests, which are sent with the token, before it is renewed again.
const RENEW_AFTER_SHARE: u32 = 2;

/// Context of the admin, who started a migration, whose token is renewed before it expires.
///
/// A migration runs in the background and its transfer can take much longer than the lifetime
/// of a token, but the sakura-hosts accept their internal endpoints only with a valid token.
pub struct Session {
    context: UserContext,
    /// Point in time, when the current token was created
    renewed_at: Instant,
    /// Lifetime of the current token
    lifetime: Duration,
}

impl Session {
    /// Starts a session with a new token for the user of the context.
    ///
    /// The token of the request is renewed right away, because its remaining lifetime is
    /// unknown, and to fail before the migration started, if miko doesn't renew it.
    ///
    /// # Arguments
    /// * `context` - Context of the request, which started the migration
    ///
    /// # Returns
    /// * `Ok(Session)` with a new token
    /// * `Err(ErrorResponse)` if miko didn't renew the token
    pub async fn start(context: UserContext) -> Result<Self, ErrorResponse> {
        let mut session = Self {
            context,
            renewed_at: Instant::now(),
            lifetime: Duration::ZERO,
        };
        session.renew().await?;
        Ok(session)
    }

    /// Returns the context with a token, which is valid for at least half of its lifetime.
    ///
    /// # Returns
    /// * `Ok(&UserContext)` with a valid token
    /// * `Err(ErrorResponse)` if the token had to be renewed and miko didn't renew it
    pub async fn context(&mut self) -> Result<&UserContext, ErrorResponse> {
        if self.renewed_at.elapsed() >= self.lifetime / RENEW_AFTER_SHARE {
            self.renew().await?;
        }
        Ok(&self.context)
    }

    /// Replaces the token of the context by a new one.
    ///
    /// The other values of the context are kept, because the sakura-hosts and gateways read
    /// them from the token itself.
    async fn renew(&mut self) -> Result<(), ErrorResponse> {
        let renewed_at = Instant::now();
        let new_token = renew_token(
            &config::CONFIG.miko.address,
            &self.context.token,
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response)?;

        self.context.token = new_token.access_token;
        self.renewed_at = renewed_at;
        self.lifetime = Duration::from_secs(new_token.expires);
        log::debug!(
            "Renewed token of migration-session of user '{}', valid for {}s",
            self.context.user_id,
            new_token.expires
        );
        Ok(())
    }
}
