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

use actix_web::{
    FromRequest,
    dev::{Payload, ServiceRequest},
};
use once_cell::sync::Lazy;
use tokio::sync::OnceCell;

use crate::auth_middleware::{ApiValidationConfig, is_internal_endpoint};

use ainari_api_structs::log_structs::AuditLogAddInternalReq;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::logging::add_audit_log;
use ainari_common::config::{Endpoint, MikoEndpoint};

/// Maximum length of the endpoint of an audit-log entry, which is accepted by omikuji.
const MAX_AUDIT_ENDPOINT_LENGTH: usize = 1024;

/// Name of the host, which is written into the audit-log entries.
static HOST_NAME: Lazy<String> = Lazy::new(|| {
    sys_info::hostname().unwrap_or_else(|e| {
        log::warn!("Failed to get the hostname for the audit-log: {e}");
        String::new()
    })
});

/// Internal address of omikuji, which was requested from miko. It is only requested once, but
/// requested again with the next request, as long as miko could not be reached. Is `None`, if
/// miko has no omikuji configured.
static OMIKUJI_ADDRESS: OnceCell<Option<String>> = OnceCell::const_new();

/// Resolves the internal address of omikuji.
///
/// # Arguments
///
/// * `miko_address` - Address of miko, which is asked for the address of omikuji
/// * `omikuji_address` - Address of omikuji from the config, which is used instead of asking miko
/// * `skip_tls_verification` - Flag to skip TLS verification
///
/// # Returns
///
/// The internal address of omikuji, or `None`, if no omikuji is configured or miko could not be
/// reached.
async fn resolve_omikuji_address(
    miko_address: &str,
    omikuji_address: Option<&str>,
    skip_tls_verification: bool,
) -> Option<String> {
    let address = match omikuji_address {
        Some(address) => Some(address.to_string()),
        None => OMIKUJI_ADDRESS
            .get_or_try_init(|| async {
                let miko_endpoint = MikoEndpoint {
                    address: miko_address.to_string(),
                };
                get_endpoints(&miko_endpoint, skip_tls_verification)
                    .await
                    .map(|endpoints| Some(endpoints.omikuji.internal_address))
            })
            .await
            .map_err(|e| log::warn!("Failed to get the address of omikuji from miko: {e}"))
            .ok()?
            .clone(),
    };

    address.filter(|address| !address.is_empty())
}

/// Sends an entry with the method and the uri of a request to the audit-log of omikuji.
///
/// The entry is sent in the background, so the request is neither delayed nor failed by omikuji.
/// Nothing is sent, if the audit-log is not enabled in the config or no omikuji is configured.
///
/// # Arguments
///
/// * `req` - The request, which is audited
/// * `api_validation_config` - Configuration of the audit-log
pub async fn send_audit_log(req: &ServiceRequest, api_validation_config: &ApiValidationConfig) {
    let Some(component_name) = &api_validation_config.audit_component_name else {
        return;
    };
    if !api_validation_config.audit_internal_endpoints && is_internal_endpoint(req.path()) {
        return;
    }

    // the token was already validated, so it is only decoded here. Requests without token get an
    // empty context.
    let context = match UserContext::from_request(req.request(), &mut Payload::None).await {
        Ok(context) => context,
        Err(_) => return,
    };

    let mut endpoint = req.uri().to_string();
    if endpoint.len() > MAX_AUDIT_ENDPOINT_LENGTH {
        let mut end = MAX_AUDIT_ENDPOINT_LENGTH;
        while !endpoint.is_char_boundary(end) {
            end -= 1;
        }
        endpoint.truncate(end);
    }

    let entry = AuditLogAddInternalReq {
        request_type: req.method().to_string(),
        endpoint,
        component_name: component_name.clone(),
        host_name: HOST_NAME.clone(),
        user_id: context.user_id,
        project_id: context.project_id,
    };

    let miko_address = api_validation_config.miko_address.clone();
    let omikuji_address = api_validation_config.omikuji_address.clone();
    let internal_api_key = api_validation_config.internal_api_key.clone();
    let skip_tls_verification = api_validation_config.skip_tls_verification;

    actix_web::rt::spawn(async move {
        let Some(internal_address) = resolve_omikuji_address(
            &miko_address,
            omikuji_address.as_deref(),
            skip_tls_verification,
        )
        .await
        else {
            return;
        };

        let omikuji_endpoint = Endpoint {
            public_address: String::new(),
            internal_address,
        };
        if let Err(e) = add_audit_log(
            &omikuji_endpoint,
            &internal_api_key,
            &entry,
            skip_tls_verification,
        )
        .await
        {
            log::warn!("Failed to send audit-log entry to omikuji: {e}");
        }
    });
}
