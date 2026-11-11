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

use actix_web::dev::Server;
use actix_web::middleware::{Logger, from_fn};
use actix_web::web;
use actix_web::{App, HttpServer};
use apistos::app::OpenApiWrapper;
use apistos::info::Info;
use apistos::info::{Contact, License};
use apistos::paths::ExternalDocumentation;
use apistos::spec::Spec;
use std::net::TcpListener;

use ainari_api::auth_middleware::*;
use ainari_api::cors_middleware::cors_middleware;
use ainari_api::endpoints::get_version_v1_0::GIT_VERSION;

use crate::config;
use crate::core::backend::Backend;

use super::routes::v1alpha::v1alpha_routes;

/// Starts the http-server of the onsen.
///
/// The server is bound to a public and an internal address. All endpoints, which access the files,
/// are internal and only reachable over the internal address, so the public address only provides
/// the version and the ready-status. Like for all other components, every request needs the token
/// of a user, which is validated by miko, and the internal endpoints also the internal API-key.
///
/// # Arguments
///
/// * `backend` - Storage-backend, where the files are stored
///
/// # Returns
///
/// `Ok(())` after the server was stopped, or the error, which made the server fail to start.
pub async fn run_server(backend: Backend) -> std::io::Result<()> {
    log::debug!("initialize server");

    // get server-address from config
    let api = &config::CONFIG.api;
    log::info!(
        "HTTP-server listen public on {}:{}",
        api.public_ip,
        api.public_port
    );
    log::info!(
        "HTTP-server listen internally on {}:{}",
        api.internal_ip,
        api.internal_port
    );
    let public = TcpListener::bind((api.public_ip.as_str(), api.public_port))?;
    let internal = TcpListener::bind((api.internal_ip.as_str(), api.internal_port))?;

    let api_validation_config = ApiValidationConfig::new(
        &config::CONFIG.miko,
        api,
        &config::INTERNAL_API_KEY,
        config::CONFIG.skip_tls_verification,
    )
    // the internal endpoints are only reachable over the internal port
    .restrict_internal_endpoints(api.internal_port);

    create_server(public, internal, api_validation_config, backend)?.await
}

/// Creates the http-server of the onsen on already bound sockets.
///
/// # Arguments
///
/// * `public` - Socket of the public address
/// * `internal` - Socket of the internal address
/// * `api_validation_config` - Settings of the authorization of the requests
/// * `backend` - Storage-backend, where the files are stored
///
/// # Returns
///
/// The server, which runs, until it is awaited and stopped, or the error, if the sockets can't be
/// used.
pub fn create_server(
    public: TcpListener,
    internal: TcpListener,
    api_validation_config: ApiValidationConfig,
    backend: Backend,
) -> std::io::Result<Server> {
    let backend = web::Data::new(backend);

    // init server with openapi-docu-generator
    let server = HttpServer::new(move || {
        let spec = Spec {
            info: Info {
                title: "Onsen-API-Documentation".to_string(),
                contact: Some(Contact {
                    email: Some("tobias.anker@kitsunemimi.moe".to_string()),
                    ..Default::default()
                }),
                license: Some(License {
                    name: "Apache 2.0".to_string(),
                    url: Some("http://www.apache.org/licenses/LICENSE-2.0.html".to_string()),
                    ..Default::default()
                }),
                version: GIT_VERSION.clone(),
                ..Default::default()
            },
            external_docs: Some(ExternalDocumentation {
                description: Some("Find out more about Swagger".to_string()),
                url: "http://swagger.io".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        App::new()
            .document(spec)
            .app_data(web::Data::new(api_validation_config.clone())) // provide validation configs to the middleware
            .app_data(backend.clone())
            .wrap(from_fn(authorization_middleware))
            .wrap(from_fn(cors_middleware))
            .wrap(Logger::default())
            .service(v1alpha_routes())
            .build("/openapi.json")
    })
    .listen(public)?
    .listen(internal)?
    .run();

    Ok(server)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LocalStorage;
    use crate::core::backend::local::LocalBackend;
    use actix_web::{HttpRequest, HttpResponse};
    use ainari_api_structs::auth_structs::UserTokenValidateResp;
    use ainari_api_structs::storage_structs::{StorageBackendType, VolumeConnection, VolumeState};
    use ainari_api_structs::user_context::UserContext;
    use ainari_clients::storage::*;
    use ainari_clients::volume::*;
    use ainari_common::config::{Api, MikoEndpoint};
    use ainari_common::constants::CHUNK_SIZE;
    use ainari_common::error::AinariError;
    use ainari_common::secret::Secret;
    use std::path::PathBuf;
    use uuid::Uuid;

    /// Onsen on free local ports with a temporary storage-location, which is deleted at the end
    /// of the test.
    struct TestOnsen {
        /// address of the internal api
        address: String,
        /// address of the public api
        public_address: String,
        storage: PathBuf,
    }

    impl Drop for TestOnsen {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.storage);
        }
    }

    const API_KEY: &str = "test-internal-api-key";

    /// Token of a user, which the fake miko accepts. Its payload is the context of the user, like
    /// in the tokens of miko, but it has no valid signature, because only miko checks it.
    const TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
        eyJ1c2VyX2lkIjoidGVzdC11c2VyIiwicHJvamVjdF9pZCI6InRlc3QtcHJvamVjdCIsImlzX2FkbWluIjoiZmFsc2UiLCJwcm9qZWN0X3JvbGUiOiJtZW1iZXIifQ.\
        signature";

    /// Token-validation of the fake miko, which only accepts `TOKEN`.
    async fn validate_token(req: HttpRequest) -> HttpResponse {
        let authorization = req
            .headers()
            .get("Authorization")
            .and_then(|value| value.to_str().ok());
        if authorization != Some(format!("Bearer {TOKEN}").as_str()) {
            return HttpResponse::Unauthorized().finish();
        }
        HttpResponse::Ok().json(UserTokenValidateResp {
            context: UserContext {
                token: TOKEN.to_string(),
                user_id: "test-user".to_string(),
                project_id: "test-project".to_string(),
                is_admin: false.to_string(),
                project_role: "member".to_string(),
            },
            project_locked: false,
        })
    }

    /// Starts a fake miko on a free local port, which only validates the tokens, and returns its
    /// address.
    fn start_miko() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let server =
            HttpServer::new(|| App::new().route("/v1alpha/token", web::get().to(validate_token)))
                .workers(1)
                .listen(listener)
                .unwrap()
                .run();
        actix_rt::spawn(server);
        address
    }

    /// Starts an onsen on free local ports, which validates the tokens against a fake miko and
    /// supports volumes.
    async fn start_onsen() -> TestOnsen {
        start_onsen_with_volumes(true).await
    }

    /// Starts an onsen on free local ports, which validates the tokens against a fake miko.
    async fn start_onsen_with_volumes(supports_volumes: bool) -> TestOnsen {
        let public = TcpListener::bind("127.0.0.1:0").unwrap();
        let internal = TcpListener::bind("127.0.0.1:0").unwrap();
        let public_address = format!("http://{}", public.local_addr().unwrap());
        let address = format!("http://{}", internal.local_addr().unwrap());
        let api = Api {
            public_ip: "127.0.0.1".to_string(),
            public_port: public.local_addr().unwrap().port(),
            internal_ip: "127.0.0.1".to_string(),
            internal_port: internal.local_addr().unwrap().port(),
        };
        let miko = MikoEndpoint {
            address: start_miko(),
        };
        let config = ApiValidationConfig::new(&miko, &api, &Secret::from(API_KEY), true)
            .restrict_internal_endpoints(api.internal_port);

        let storage = std::env::temp_dir().join(format!("onsen_test_{}", Uuid::new_v4()));
        let backend = LocalBackend::new(&LocalStorage {
            location: storage.to_string_lossy().to_string(),
            nfs_server: supports_volumes.then(|| "10.0.0.5".to_string()),
            nfs_export: supports_volumes.then(|| "/srv/onsen/volumes".to_string()),
        })
        .await
        .unwrap();
        let server = create_server(public, internal, config, Backend::Local(backend)).unwrap();
        actix_rt::spawn(server);

        TestOnsen {
            address,
            public_address,
            storage,
        }
    }

    #[actix_rt::test]
    async fn test_transfer_files() {
        let onsen = start_onsen().await;
        let key = Secret::from(API_KEY);
        let address = onsen.address.as_str();

        // the content spans multiple chunks
        let content: Vec<u8> = (0..CHUNK_SIZE * 2 + 123).map(|i| (i % 251) as u8).collect();
        upload_data(address, TOKEN, &key, "dir/data.bin", content.clone(), false)
            .await
            .unwrap();
        assert_eq!(
            download_data(address, TOKEN, &key, "dir/data.bin", content.len(), false)
                .await
                .unwrap(),
            content
        );

        // a file, which is larger than allowed, is not read into memory
        assert!(matches!(
            download_data(
                address,
                TOKEN,
                &key,
                "dir/data.bin",
                content.len() - 1,
                false
            )
            .await,
            Err(AinariError::InvalidInput(_))
        ));

        // upload and download of local files
        let local_dir = onsen.storage.join("local");
        std::fs::create_dir_all(&local_dir).unwrap();
        let local_source = local_dir.join("source.bin");
        let local_target = local_dir.join("target.bin");
        std::fs::write(&local_source, &content).unwrap();
        upload_file(
            address,
            TOKEN,
            &key,
            "dir/file.bin",
            local_source.to_str().unwrap(),
            false,
        )
        .await
        .unwrap();
        download_file(
            address,
            TOKEN,
            &key,
            "dir/file.bin",
            local_target.to_str().unwrap(),
            false,
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&local_target).unwrap(), content);

        let info = get_file_info(address, TOKEN, &key, "dir/file.bin", false)
            .await
            .unwrap();
        assert_eq!(info.size, content.len() as u64);
        assert_eq!(info.sha256.len(), 64);

        let entries = list_directory(address, TOKEN, &key, "dir", false)
            .await
            .unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["data.bin", "file.bin"]);

        delete_file(address, TOKEN, &key, "dir/data.bin", false)
            .await
            .unwrap();
        assert!(matches!(
            download_data(address, TOKEN, &key, "dir/data.bin", 10, false).await,
            Err(AinariError::NotFound(_))
        ));

        delete_directory(address, TOKEN, &key, "dir", false)
            .await
            .unwrap();
        assert!(matches!(
            list_directory(address, TOKEN, &key, "dir", false).await,
            Err(AinariError::NotFound(_))
        ));

        // invalid paths are rejected
        assert!(matches!(
            upload_data(address, TOKEN, &key, "../outside.bin", vec![1], false).await,
            Err(AinariError::InvalidInput(_))
        ));
        assert!(matches!(
            delete_directory(address, TOKEN, &key, "", false).await,
            Err(AinariError::InvalidInput(_))
        ));

        // the directory of the volumes is reserved for the volume-API
        assert!(matches!(
            upload_data(address, TOKEN, &key, "volumes/file.bin", vec![1], false).await,
            Err(AinariError::InvalidInput(_))
        ));
        assert!(matches!(
            delete_directory(address, TOKEN, &key, "volumes", false).await,
            Err(AinariError::InvalidInput(_))
        ));
        assert!(onsen.storage.join("volumes").is_dir());
        let entries = list_directory(address, TOKEN, &key, "", false)
            .await
            .unwrap();
        assert!(entries.iter().all(|e| e.name != "volumes"));
    }

    #[actix_rt::test]
    async fn test_copy_file_between_onsens() {
        let source = start_onsen().await;
        let target = start_onsen().await;
        let key = Secret::from(API_KEY);

        let content: Vec<u8> = (0..CHUNK_SIZE * 2 + 123).map(|i| (i % 251) as u8).collect();
        upload_data(
            &source.address,
            TOKEN,
            &key,
            "source.bin",
            content.clone(),
            false,
        )
        .await
        .unwrap();

        let copied = copy_file(
            &source.address,
            "source.bin",
            &target.address,
            "copy/target.bin",
            TOKEN,
            &key,
            false,
        )
        .await
        .unwrap();
        assert_eq!(copied, content.len() as u64);
        assert_eq!(
            download_data(
                &target.address,
                TOKEN,
                &key,
                "copy/target.bin",
                content.len(),
                false
            )
            .await
            .unwrap(),
            content
        );

        // a missing source leaves no file on the target
        assert!(matches!(
            copy_file(
                &source.address,
                "missing.bin",
                &target.address,
                "copy/missing.bin",
                TOKEN,
                &key,
                false
            )
            .await,
            Err(AinariError::NotFound(_))
        ));
        assert!(!target.storage.join("copy/missing.bin").exists());

        // an empty file is copied as well
        upload_data(&source.address, TOKEN, &key, "source.bin", vec![], false)
            .await
            .unwrap();
        assert_eq!(
            copy_file(
                &source.address,
                "source.bin",
                &target.address,
                "copy/target.bin",
                TOKEN,
                &key,
                false
            )
            .await
            .unwrap(),
            0
        );
        assert!(
            download_data(&target.address, TOKEN, &key, "copy/target.bin", 10, false)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[actix_rt::test]
    async fn test_volumes() {
        let onsen = start_onsen().await;
        let key = Secret::from(API_KEY);
        let address = onsen.address.as_str();
        let id = Uuid::new_v4();
        const MIB: u64 = 1024 * 1024;

        let backend = get_storage_backend(address, TOKEN, &key, false)
            .await
            .unwrap();
        assert_eq!(backend.backend_type, StorageBackendType::Local);
        assert!(backend.supports_volumes);
        assert!(backend.total_capacity >= backend.free_capacity);

        // the new volume is sparse
        let info = create_volume(address, TOKEN, &key, &id, 64 * MIB, false)
            .await
            .unwrap();
        assert_eq!(info.backend_id, id);
        assert_eq!(info.state, VolumeState::Ready);
        assert_eq!(info.size, 64 * MIB);
        assert!(info.used_size < 64 * MIB);
        let file = onsen.storage.join(format!("volumes/{id}.img"));
        assert_eq!(std::fs::metadata(&file).unwrap().len(), 64 * MIB);

        // a repeated create with the same size is no error, but not with another size
        create_volume(address, TOKEN, &key, &id, 64 * MIB, false)
            .await
            .unwrap();
        assert!(matches!(
            create_volume(address, TOKEN, &key, &id, 32 * MIB, false).await,
            Err(AinariError::Conflict(_))
        ));
        assert!(matches!(
            create_volume(address, TOKEN, &key, &Uuid::new_v4(), 1000, false).await,
            Err(AinariError::InvalidInput(_))
        ));

        // a volume only grows
        let info = resize_volume(address, TOKEN, &key, &id, 128 * MIB, false)
            .await
            .unwrap();
        assert_eq!(info.size, 128 * MIB);
        assert!(matches!(
            resize_volume(address, TOKEN, &key, &id, 64 * MIB, false).await,
            Err(AinariError::InvalidInput(_))
        ));
        assert!(matches!(
            resize_volume(address, TOKEN, &key, &Uuid::new_v4(), 64 * MIB, false).await,
            Err(AinariError::NotFound(_))
        ));

        assert_eq!(
            get_volume_connection(address, TOKEN, &key, &id, false)
                .await
                .unwrap(),
            VolumeConnection::Nfs {
                server: "10.0.0.5".to_string(),
                export: "/srv/onsen/volumes".to_string(),
                file_name: format!("{id}.img"),
            }
        );

        // copy within the backend, the content is kept
        std::fs::write(&file, b"encrypted").unwrap();
        let copy_id = Uuid::new_v4();
        copy_volume(address, TOKEN, &key, &id, &copy_id, false)
            .await
            .unwrap();
        let mut info = get_volume_info(address, TOKEN, &key, &copy_id, false)
            .await
            .unwrap();
        for _ in 0..100 {
            if info.state != VolumeState::Copying {
                break;
            }
            actix_rt::time::sleep(std::time::Duration::from_millis(50)).await;
            info = get_volume_info(address, TOKEN, &key, &copy_id, false)
                .await
                .unwrap();
        }
        assert_eq!(info.state, VolumeState::Ready);
        assert_eq!(
            std::fs::read(onsen.storage.join(format!("volumes/{copy_id}.img"))).unwrap(),
            b"encrypted"
        );
        assert!(matches!(
            copy_volume(address, TOKEN, &key, &id, &copy_id, false).await,
            Err(AinariError::Conflict(_))
        ));
        assert!(matches!(
            copy_volume(
                address,
                TOKEN,
                &key,
                &Uuid::new_v4(),
                &Uuid::new_v4(),
                false
            )
            .await,
            Err(AinariError::NotFound(_))
        ));

        // delete is idempotent
        delete_volume(address, TOKEN, &key, &id, false)
            .await
            .unwrap();
        delete_volume(address, TOKEN, &key, &id, false)
            .await
            .unwrap();
        assert!(!file.exists());
        assert!(matches!(
            get_volume_info(address, TOKEN, &key, &id, false).await,
            Err(AinariError::NotFound(_))
        ));
        assert!(matches!(
            get_volume_connection(address, TOKEN, &key, &id, false).await,
            Err(AinariError::NotFound(_))
        ));
    }

    #[actix_rt::test]
    async fn test_volumes_not_supported() {
        let onsen = start_onsen_with_volumes(false).await;
        let key = Secret::from(API_KEY);
        let address = onsen.address.as_str();

        let backend = get_storage_backend(address, TOKEN, &key, false)
            .await
            .unwrap();
        assert!(!backend.supports_volumes);
        assert!(matches!(
            create_volume(address, TOKEN, &key, &Uuid::new_v4(), 1024 * 1024, false).await,
            Err(AinariError::Conflict(_))
        ));
        assert!(!onsen.storage.join("volumes").exists());

        // the files work like before
        upload_data(address, TOKEN, &key, "file.bin", vec![1], false)
            .await
            .unwrap();
    }

    #[actix_rt::test]
    async fn test_openapi_spec() {
        let onsen = start_onsen().await;
        let client = ainari_clients::prepare_client(&onsen.public_address, false);
        let mut response = client
            .get(format!("{}/openapi.json", onsen.public_address))
            .send()
            .await
            .unwrap();
        let spec: serde_json::Value = response.json().limit(1 << 20).await.unwrap();

        let paths = spec["paths"].as_object().unwrap();
        for (path, methods) in [
            ("/v1alpha/file/internal", vec!["put", "get", "delete"]),
            ("/v1alpha/file/info/internal", vec!["get"]),
            ("/v1alpha/directory/internal", vec!["get", "delete"]),
            ("/v1alpha/volume/internal", vec!["post"]),
            (
                "/v1alpha/volume/{backend_id}/internal",
                vec!["get", "delete"],
            ),
            ("/v1alpha/volume/{backend_id}/size/internal", vec!["put"]),
            ("/v1alpha/volume/{backend_id}/copy/internal", vec!["post"]),
            (
                "/v1alpha/volume/{backend_id}/connection/internal",
                vec!["get"],
            ),
            ("/v1alpha/backend/internal", vec!["get"]),
        ] {
            for method in methods {
                assert!(
                    paths[path].get(method).is_some(),
                    "{method} {path} is not documented"
                );
            }
        }
        assert!(
            paths["/v1alpha/file/internal"]["put"]["requestBody"]["content"]
                .get("application/octet-stream")
                .is_some()
        );
    }

    #[actix_rt::test]
    async fn test_internal_endpoints_are_protected() {
        let onsen = start_onsen().await;

        // a wrong internal API-key
        assert!(matches!(
            upload_data(
                &onsen.address,
                TOKEN,
                &Secret::from("wrong"),
                "file.bin",
                vec![1],
                false
            )
            .await,
            Err(AinariError::Unauthorized(_))
        ));

        // a missing or an invalid token of the user
        assert!(matches!(
            upload_data(
                &onsen.address,
                "",
                &Secret::from(API_KEY),
                "file.bin",
                vec![1],
                false
            )
            .await,
            Err(AinariError::Unauthorized(_))
        ));
        assert!(matches!(
            list_directory(
                &onsen.address,
                "invalid.token.signature",
                &Secret::from(API_KEY),
                "",
                false
            )
            .await,
            Err(AinariError::Unauthorized(_))
        ));

        // the internal endpoints are not reachable over the public port
        assert!(matches!(
            upload_data(
                &onsen.public_address,
                TOKEN,
                &Secret::from(API_KEY),
                "file.bin",
                vec![1],
                false
            )
            .await,
            Err(AinariError::Forbidden(_))
        ));
        assert!(!onsen.storage.join("file.bin").exists());
    }
}
