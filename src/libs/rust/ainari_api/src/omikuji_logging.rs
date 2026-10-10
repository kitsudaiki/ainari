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

use std::rc::Rc;
use std::sync::OnceLock;
use std::thread;

use actix_web::{
    FromRequest,
    dev::{Payload, ServiceRequest},
};
use once_cell::sync::Lazy;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::auth_middleware::is_internal_endpoint;

use ainari_api_structs::log_structs::{AuditLogAddInternalReq, ErrorLogAddInternalReq};
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::logging::{add_audit_log, add_error_log};
use ainari_common::config::{Endpoint, MikoEndpoint};
use ainari_common::error::InternalErrorDetails;
use ainari_common::secret::Secret;

/// Maximum length of the endpoint of an audit-log entry, which is accepted by omikuji.
const MAX_AUDIT_ENDPOINT_LENGTH: usize = 1024;

/// Maximum length of the message of an error-log entry, which is accepted by omikuji.
const MAX_ERROR_MESSAGE_LENGTH: usize = 65535;

/// Name of the host, which is written into the log-entries.
static HOST_NAME: Lazy<String> = Lazy::new(|| {
    sys_info::hostname().unwrap_or_else(|e| {
        log::warn!("Failed to get the hostname for the omikuji-logs: {e}");
        String::new()
    })
});

/// Connection to the sender-thread, which is set by `OmikujiLogging::init`. As long as it is not
/// set, no entries are sent to omikuji.
static LOGGING: OnceLock<Logging> = OnceLock::new();

/// Settings for sending the audit- and error-logs of a component to omikuji.
#[derive(Debug, Clone)]
pub struct OmikujiLogging {
    /// Name of the component, which is written into the entries
    component_name: String,
    /// Address of miko, which is asked for the address of omikuji
    miko_address: String,
    /// Internal address of omikuji. If not set, it is requested from miko.
    omikuji_address: Option<String>,
    /// Secret API key for the internal endpoints of omikuji
    internal_api_key: Secret,
    /// Flag to skip TLS verification when communicating with miko and omikuji
    skip_tls_verification: bool,
    /// If false, the requests against internal endpoints are not sent to the audit-log.
    audit_internal_endpoints: bool,
}

/// Connection of the component to the sender-thread.
struct Logging {
    sender: UnboundedSender<LogEntry>,
    component_name: String,
    audit_internal_endpoints: bool,
}

/// Entry, which is queued for the sender-thread.
enum LogEntry {
    Audit(AuditLogAddInternalReq),
    Error(ErrorLogAddInternalReq),
}

/// Information of a request, which is written into the log-entries.
#[derive(Debug, Clone)]
pub struct RequestInfo {
    method: String,
    endpoint: String,
    user_id: String,
    project_id: String,
    is_internal: bool,
}

impl OmikujiLogging {
    /// Creates the settings for sending the logs of a component to omikuji.
    ///
    /// # Arguments
    ///
    /// * `component_name` - Name of the component, which is written into the entries
    /// * `miko_endpoint` - Endpoint of miko, which is asked for the address of omikuji
    /// * `internal_api_key` - Secret API key for the internal endpoints of omikuji
    /// * `skip_tls_verification` - Flag to skip TLS verification
    ///
    /// # Returns
    ///
    /// The new settings
    pub fn new(
        component_name: &str,
        miko_endpoint: &MikoEndpoint,
        internal_api_key: &Secret,
        skip_tls_verification: bool,
    ) -> Self {
        OmikujiLogging {
            component_name: component_name.to_string(),
            miko_address: miko_endpoint.address.clone(),
            omikuji_address: None,
            internal_api_key: internal_api_key.clone(),
            skip_tls_verification,
            audit_internal_endpoints: true,
        }
    }

    /// Sets the internal address of omikuji, instead of requesting it from miko.
    ///
    /// Required for miko itself, which can not request the address from itself.
    ///
    /// # Arguments
    ///
    /// * `omikuji_endpoint` - Endpoint of omikuji. An empty internal address disables the logs.
    ///
    /// # Returns
    ///
    /// The settings with the address of omikuji
    pub fn with_omikuji_endpoint(mut self, omikuji_endpoint: &Endpoint) -> Self {
        self.omikuji_address = Some(omikuji_endpoint.internal_address.clone());
        self
    }

    /// Doesn't send the requests against internal endpoints to the audit-log.
    ///
    /// Required for omikuji itself, where the entries are added over the internal endpoints, so
    /// each added entry would add another entry.
    ///
    /// # Returns
    ///
    /// The settings without audit-log for the internal endpoints
    pub fn skip_internal_endpoints(mut self) -> Self {
        self.audit_internal_endpoints = false;
        self
    }

    /// Starts sending the logs to omikuji.
    ///
    /// The entries are sent by a separate thread, so they can be created everywhere, also outside
    /// of an async runtime, and neither delay nor fail the code, which creates them. Can only be
    /// called once.
    pub fn init(self) {
        let (sender, receiver) = unbounded_channel();
        let logging = Logging {
            sender,
            component_name: self.component_name.clone(),
            audit_internal_endpoints: self.audit_internal_endpoints,
        };
        if LOGGING.set(logging).is_err() {
            log::warn!("The logging to omikuji was already initialized");
            return;
        }

        let result = thread::Builder::new()
            .name("omikuji-logging".to_string())
            .spawn(move || actix_rt::System::new().block_on(run_sender(self, receiver)));
        if let Err(e) = result {
            log::warn!("Failed to start the thread for the logging to omikuji: {e}");
        }
    }
}

impl RequestInfo {
    /// Collects the information of a request for the log-entries.
    ///
    /// # Arguments
    ///
    /// * `req` - The request
    ///
    /// # Returns
    ///
    /// The information of the request
    pub async fn from_request(req: &ServiceRequest) -> Self {
        // the token was already validated, so it is only decoded here. Requests without token get
        // an empty context.
        let (user_id, project_id) =
            match UserContext::from_request(req.request(), &mut Payload::None).await {
                Ok(context) => (context.user_id, context.project_id),
                Err(_) => (String::new(), String::new()),
            };

        RequestInfo {
            method: req.method().to_string(),
            endpoint: truncate(req.uri().to_string(), MAX_AUDIT_ENDPOINT_LENGTH),
            user_id,
            project_id,
            is_internal: is_internal_endpoint(req.path()),
        }
    }
}

/// Cuts a string to a maximum number of bytes, without splitting a character.
fn truncate(mut value: String, max_length: usize) -> String {
    if value.len() > max_length {
        let mut end = max_length;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value.truncate(end);
    }
    value
}

/// Sends an entry with the method and the uri of a request to the audit-log of omikuji.
///
/// # Arguments
///
/// * `request_info` - Information of the request, which is audited
pub fn send_audit_log(request_info: &RequestInfo) {
    let Some(logging) = LOGGING.get() else {
        return;
    };
    if request_info.is_internal && !logging.audit_internal_endpoints {
        return;
    }

    let entry = AuditLogAddInternalReq {
        request_type: request_info.method.clone(),
        endpoint: request_info.endpoint.clone(),
        component_name: logging.component_name.clone(),
        host_name: HOST_NAME.clone(),
        user_id: request_info.user_id.clone(),
        project_id: request_info.project_id.clone(),
    };
    // the receiver only stops together with the process
    let _ = logging.sender.send(LogEntry::Audit(entry));
}

/// Writes an internal error into the local log and sends it to the error-log of omikuji.
///
/// # Arguments
///
/// * `details` - The error, together with the user and project, which it is assigned to
pub fn report_internal_error(details: InternalErrorDetails) {
    log::error!("{}", details.message);

    let Some(logging) = LOGGING.get() else {
        return;
    };
    let entry = ErrorLogAddInternalReq {
        message: truncate(details.message, MAX_ERROR_MESSAGE_LENGTH),
        component_name: logging.component_name.clone(),
        host_name: HOST_NAME.clone(),
        user_id: details.user_id,
        project_id: details.project_id,
    };
    // the receiver only stops together with the process
    let _ = logging.sender.send(LogEntry::Error(entry));
}

/// Same as `report_internal_error`, but the error is assigned to the user and project of a
/// context. Normally used by the `report_internal_error`-macro.
///
/// # Arguments
///
/// * `context` - Context of the request, which caused the error, or `None` outside of a request
/// * `message` - The error-message
pub fn report_internal_error_of_context(context: Option<&UserContext>, message: String) {
    let details = InternalErrorDetails::new(message);
    match context {
        Some(context) => {
            report_internal_error(details.with_ids(&context.user_id, &context.project_id))
        }
        None => report_internal_error(details),
    }
}

/// Writes an internal error into the local log and sends it to the error-log of omikuji.
///
/// The first argument is the context of the request, which caused the error, as
/// `Option<&UserContext>`, so the error is assigned to its user and project. Outside of a request
/// it is `None`. The other arguments are the same as for `log::error!`.
///
/// ```ignore
/// report_internal_error!(Some(&context), "Failed to list secrets: {e}");
/// report_internal_error!(None, "Failed to restore host '{host}'");
/// ```
#[macro_export]
macro_rules! report_internal_error {
    ($context:expr, $($arg:tt)+) => {
        $crate::omikuji_logging::report_internal_error_of_context($context, format!($($arg)+))
    };
}

/// Sends the queued entries to omikuji, until the process ends.
///
/// Failures are only logged as warnings and not reported as internal errors, because else a
/// missing omikuji would produce more and more entries.
///
/// # Arguments
///
/// * `settings` - Settings of the logging
/// * `receiver` - Queue of the entries
async fn run_sender(settings: OmikujiLogging, mut receiver: UnboundedReceiver<LogEntry>) {
    let settings = Rc::new(settings);
    // requested from miko with the first entry, and again as long as miko could not be reached
    let mut omikuji_address = settings.omikuji_address.clone();

    while let Some(entry) = receiver.recv().await {
        if omikuji_address.is_none() {
            let miko_endpoint = MikoEndpoint {
                address: settings.miko_address.clone(),
            };
            match get_endpoints(&miko_endpoint, settings.skip_tls_verification).await {
                Ok(endpoints) => omikuji_address = Some(endpoints.omikuji.internal_address),
                Err(e) => {
                    log::warn!("Failed to get the address of omikuji from miko: {e}");
                    continue;
                }
            }
        }

        // miko has no omikuji configured
        let internal_address = omikuji_address.clone().unwrap_or_default();
        if internal_address.is_empty() {
            continue;
        }

        let settings = settings.clone();
        actix_rt::spawn(async move {
            let omikuji_endpoint = Endpoint {
                public_address: String::new(),
                internal_address,
            };
            let api_key = &settings.internal_api_key;
            let skip_tls = settings.skip_tls_verification;
            let result = match &entry {
                LogEntry::Audit(entry) => {
                    add_audit_log(&omikuji_endpoint, api_key, entry, skip_tls)
                        .await
                        .map(|_| ())
                }
                LogEntry::Error(entry) => {
                    add_error_log(&omikuji_endpoint, api_key, entry, skip_tls)
                        .await
                        .map(|_| ())
                }
            };
            if let Err(e) = result {
                log::warn!("Failed to send log-entry to omikuji: {e}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("abc".to_string(), 5), "abc");
        assert_eq!(truncate("abcdef".to_string(), 3), "abc");
        // 'ä' takes 2 bytes, so it is not split
        assert_eq!(truncate("aä".to_string(), 2), "a");
    }
}
