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

use actix_multipart::{Field, Multipart};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use apistos::actix::CreatedJson;
use apistos::reference_or::ReferenceOr;
use apistos::{ApiComponent, api_operation};
use futures_util::StreamExt;
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use std::future::{Ready, ready};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::database::image_table;
use crate::onsen_functions::select_onsen;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::image_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_files::file_encryption::encrypt_file;
use ainari_files::file_info::get_disk_image_info;

/// Maximum size of a text-field of the multipart-form in bytes
const MAX_TEXT_FIELD_SIZE: usize = 1024;

// It is only used for the openapi-documentation, because the file is streamed directly to disk,
// instead of collecting it in memory.
/// Multipart-form to upload a new image
#[derive(JsonSchema, ApiComponent)]
#[allow(dead_code)]
struct ImageUploadForm {
    #[schemars(flatten)]
    fields: ImageUploadReq,
    /// The disk-image, which is used as boot-disk of a virtual machine
    #[schemars(schema_with = "binary_file_schema")]
    file: String,
}

/// Returns the openapi-schema of an uploaded file
fn binary_file_schema(_: &mut SchemaGenerator) -> Schema {
    SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        format: Some("binary".to_string()),
        ..Default::default()
    }
    .into()
}

/// Multipart-payload of the image-upload, which documents the fields of the form in the openapi-
/// specification. A plain `Multipart` is documented without any fields.
pub struct ImageUploadPayload(Multipart);

impl FromRequest for ImageUploadPayload {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        ready(Ok(Self(Multipart::new(req.headers(), payload.take()))))
    }
}

impl ApiComponent for ImageUploadPayload {
    fn content_type() -> String {
        "multipart/form-data".to_string()
    }

    fn child_schemas() -> Vec<(String, ReferenceOr<apistos::Schema>)> {
        ImageUploadForm::child_schemas()
    }

    fn schema() -> Option<(String, ReferenceOr<apistos::Schema>)> {
        ImageUploadForm::schema()
    }
}

#[api_operation(
    tag = "image",
    summary = "Create new image",
    description = r###"Create new image by uploading a multipart-form with the fields `name`,
`image_type` (at the moment only `disk`), the optional `secret_uuid` and the disk-image as `file`.
With `secret_uuid`, the image is encrypted with this already existing secret of the omamori,
instead of a new generated secret.

The image is registered with the status `ENCRYPTING`, when the file is completely uploaded, while
ryokan encrypts the file. While the encrypted file is stored in the onsen, the status is
`UPLOADING`. Afterwards the image is `READY` and can be used for virtual machines. If the encryption
or the storing fails, the image is removed again.

Before the file is encrypted, its type (e.g. `qcow2` or `raw`) is identified and its size is
measured. Both are returned as `file_type` and `file_size`."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn upload_binary(
    payload: ImageUploadPayload,
    context: UserContext,
) -> Result<CreatedJson<ImageResp>, ErrorResponse> {
    let image_uuid = Uuid::new_v4();
    let target_dir_path = format!(
        "{}/{}",
        config::CONFIG.storage.tempfile_location,
        image_uuid
    );
    let source_path = format!("{target_dir_path}/upload.bin");
    let encrypted_result_path = format!("{target_dir_path}/encrypted_result");
    let upload_file_path_str: String = format!("images/{image_uuid}");

    super::check_image_quota(&context).await?;

    create_directory_api(&target_dir_path, Some(&context)).await?;

    let selected_onsen = select_onsen(&context)?;

    // handle payload, create key encrypt data and upload them to the selected onsen
    let result = async {
        // a disk-image is the boot-disk of a virtual_machine, which is stored as it is, so there
        // is nothing to convert
        let req = read_upload_form(payload.0, &source_path).await?;

        req.validate()
            .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
        super::check_image_type(&req.image_type)?;

        // the type and size can only be measured on the unencrypted file
        let file_info = get_disk_image_info(&source_path)
            .await
            .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

        // use the secret provided by the user or generate a new one for the image
        let (secret_uuid, secret) = match &req.secret_uuid {
            Some(secret_uuid) => {
                super::super::get_existing_key(secret_uuid, "image", &image_uuid, &context).await?
            }
            None => {
                super::super::generate_new_key(&image_uuid, "image", &image_uuid, &context).await?
            }
        };

        // the image is registered, while it is encrypted and stored, so it is visible, but can not
        // be used yet
        let stored = async {
            image_table::add_new_image(
                &image_uuid,
                &req.name,
                &selected_onsen.address,
                &upload_file_path_str,
                &file_info.file_type,
                file_info.file_size as i64,
                &secret_uuid,
                false,
                &ImageStatus::Encrypting,
                &context,
            )
            .map_err(|e| map_db_write_error("add image to database", e, Some(&context)))?;

            encrypt_file(&source_path, &encrypted_result_path, &secret)
                .await
                .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

            image_table::update_image_status(&image_uuid, &ImageStatus::Uploading, None, &context)
                .map_err(|e| {
                    map_db_uuid_get_delete_error("image", &image_uuid, e, Some(&context))
                })?;

            upload_file_to_onsen(
                &selected_onsen.address,
                &upload_file_path_str,
                &encrypted_result_path,
                Some(&context),
            )
            .await?;

            image_table::update_image_status(&image_uuid, &ImageStatus::Ready, None, &context)
                .map_err(|e| map_db_uuid_get_delete_error("image", &image_uuid, e, Some(&context)))
        }
        .await;

        // an incomplete image can never become ready, so it is removed together with its secret
        if stored.is_err() {
            super::remove_failed_image(
                &image_uuid,
                &selected_onsen.address,
                &upload_file_path_str,
                &secret_uuid,
                &context,
            )
            .await;
        }
        stored
    }
    .await;

    // remove temporary directory again
    super::remove_all(&target_dir_path);

    result?;

    let image_data = image_table::get_image(&image_uuid, &context)
        .map_err(|e| map_db_uuid_get_after_add_error("image", &image_uuid, e, Some(&context)))?;

    let image_status = super::image_status_of(&image_data)?;
    let resp = ImageResp {
        uuid: image_uuid,
        name: image_data.name,
        file_type: image_data.file_type,
        file_size: image_data.file_size as u64,
        image_status,
        is_snapshot: image_data.is_snapshot,
        created_by: image_data.created_by,
        created_at: image_data.created_at,
        updated_by: image_data.updated_by,
        updated_at: image_data.updated_at,
    };

    Ok(CreatedJson(resp))
}

/// Reads the multipart-form of the image-upload.
///
/// The text-fields are collected into the request and the uploaded file is written to the given
/// path, so also large disk-images don't have to be kept in memory.
///
/// # Arguments
///
/// * `payload` - The multipart data to process
/// * `file_path` - Path, where the uploaded file is written to
///
/// # Returns
///
/// A `Result` containing the fields of the form, or an `ErrorResponse`, if the form is not valid
/// or the file could not be written
async fn read_upload_form(
    mut payload: Multipart,
    file_path: &str,
) -> Result<ImageUploadReq, ErrorResponse> {
    let mut req = ImageUploadReq::default();
    let mut has_name = false;
    let mut has_image_type = false;
    let mut has_file = false;

    // process items from payload
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|_| {
            ErrorResponse::BadRequest("Failed to read next item from input.".to_string())
        })?;

        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "name" => {
                req.name = read_text_field(&mut field, &field_name).await?;
                has_name = true;
            }
            "image_type" => {
                req.image_type = read_text_field(&mut field, &field_name).await?;
                has_image_type = true;
            }
            "secret_uuid" => {
                let value = read_text_field(&mut field, &field_name).await?;
                // an empty field is handled like a missing one, which is easier for html-forms
                if !value.is_empty() {
                    let secret_uuid = Uuid::parse_str(&value).map_err(|_| {
                        ErrorResponse::BadRequest(format!("'{value}' is not a valid secret_uuid."))
                    })?;
                    req.secret_uuid = Some(secret_uuid);
                }
            }
            "file" => {
                if has_file {
                    return Err(ErrorResponse::BadRequest(
                        "Disk-image expect only 1 uploaded file.".to_string(),
                    ));
                }
                write_field_into_file(&mut field, file_path).await?;
                has_file = true;
            }
            _ => {
                return Err(ErrorResponse::BadRequest(format!(
                    "Unknown field '{field_name}' in the multipart-form."
                )));
            }
        }
    }

    for (is_set, field_name) in [
        (has_name, "name"),
        (has_image_type, "image_type"),
        (has_file, "file"),
    ] {
        if !is_set {
            return Err(ErrorResponse::BadRequest(format!(
                "Field '{field_name}' is missing in the multipart-form."
            )));
        }
    }

    Ok(req)
}

/// Reads a text-field of the multipart-form.
///
/// # Arguments
///
/// * `field` - The field to read
/// * `field_name` - Name of the field for the error-message
///
/// # Returns
///
/// A `Result` containing the trimmed text of the field, or an `ErrorResponse`, if the field is too
/// long or not valid utf-8
async fn read_text_field(field: &mut Field, field_name: &str) -> Result<String, ErrorResponse> {
    let mut data = Vec::new();
    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|_| {
            ErrorResponse::BadRequest(format!("Failed to read field '{field_name}'."))
        })?;
        if data.len() + chunk.len() > MAX_TEXT_FIELD_SIZE {
            return Err(ErrorResponse::BadRequest(format!(
                "Field '{field_name}' is longer than {MAX_TEXT_FIELD_SIZE} bytes."
            )));
        }
        data.extend_from_slice(&chunk);
    }

    let text = String::from_utf8(data).map_err(|_| {
        ErrorResponse::BadRequest(format!("Field '{field_name}' is not valid utf-8."))
    })?;

    Ok(text.trim().to_string())
}

/// Writes the content of a field of the multipart-form into a file.
///
/// # Arguments
///
/// * `field` - The field, which contains the uploaded file
/// * `file_path` - Path of the file to create
///
/// # Returns
///
/// `Ok(())` on success, or an `ErrorResponse`, if the file could not be written. In this case the
/// incomplete file is removed again.
async fn write_field_into_file(field: &mut Field, file_path: &str) -> Result<(), ErrorResponse> {
    // create file
    let mut f = fs::File::create(file_path).await.map_err(|e| {
        ainari_api::report_internal_error!(
            None,
            "Failed to create upload-file '{file_path}' with error: {e}."
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // fill content into file
    let result = async {
        while let Some(chunk) = field.next().await {
            let data = chunk.map_err(|e| {
                ainari_api::report_internal_error!(
                    None,
                    "Failed to fill content into file with error '{e}'"
                );
                ErrorResponse::InternalError("Internal Error".to_string())
            })?;

            f.write_all(&data).await.map_err(|e| {
                ainari_api::report_internal_error!(
                    None,
                    "Failed to write all chunks into file with error: '{e}'"
                );
                ErrorResponse::InternalError("Internal Error".to_string())
            })?;
        }

        f.sync_all().await.map_err(|e| {
            ainari_api::report_internal_error!(
                None,
                "Failed to sync file to disc with error: '{e}'"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

        Ok(())
    }
    .await;

    if result.is_err() {
        log::debug!("Image-upload broken or canceled.");
        let _ = std::fs::remove_file(file_path).map_err(|e| {
            log::error!("Failed to delete temp-file {file_path} from disc with error {e}.");
        });
    }

    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::error::PayloadError;
    use actix_web::http::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
    use bytes::Bytes;
    use std::time::Duration;

    const BOUNDARY: &str = "test-boundary";

    /// Creates a multipart-payload, which is delivered in small chunks like over the network
    fn multipart(parts: &[(&str, &[u8])]) -> Multipart {
        let mut body = Vec::new();
        for (name, content) in parts {
            body.extend_from_slice(
                format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"")
                    .as_bytes(),
            );
            if *name == "file" {
                body.extend_from_slice(
                    b"; filename=\"disk.img\"\r\nContent-Type: application/octet-stream",
                );
            }
            body.extend_from_slice(b"\r\n\r\n");
            body.extend_from_slice(content);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());

        let mut headers = HeaderMap::new();
        let content_type = format!("multipart/form-data; boundary={BOUNDARY}");
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(&content_type).unwrap());
        let chunks: Vec<Result<Bytes, PayloadError>> = body
            .chunks(1000)
            .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
            .collect();
        Multipart::new(&headers, futures_util::stream::iter(chunks))
    }

    /// Reads the form like the upload. A blocked payload makes the test fail instead of hanging.
    async fn read_form(
        payload: Multipart,
        file_path: &str,
    ) -> Result<ImageUploadReq, ErrorResponse> {
        tokio::time::timeout(
            Duration::from_secs(10),
            read_upload_form(payload, file_path),
        )
        .await
        .expect("reading the multipart-form is blocked")
    }

    fn temp_file_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("{name}-{}", Uuid::new_v4()))
            .to_string_lossy()
            .to_string()
    }

    #[actix_rt::test]
    async fn read_complete_form() {
        let content: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        let secret_uuid = Uuid::new_v4();
        let file_path = temp_file_path("upload");

        let payload = multipart(&[
            ("name", b"my-image"),
            ("image_type", b"disk"),
            ("secret_uuid", secret_uuid.to_string().as_bytes()),
            ("file", &content),
        ]);
        let Ok(req) = read_form(payload, &file_path).await else {
            panic!("valid form was rejected");
        };

        assert_eq!(req.name, "my-image");
        assert_eq!(req.image_type, "disk");
        assert_eq!(req.secret_uuid, Some(secret_uuid));
        assert_eq!(std::fs::read(&file_path).unwrap(), content);
        let _ = std::fs::remove_file(&file_path);
    }

    #[actix_rt::test]
    async fn read_form_with_file_first() {
        let file_path = temp_file_path("upload-file-first");

        // the order of the fields doesn't matter, because the form is read completely first
        let payload = multipart(&[
            ("file", b"data"),
            ("image_type", b"disk"),
            ("name", b"my-image"),
        ]);
        let Ok(req) = read_form(payload, &file_path).await else {
            panic!("valid form was rejected");
        };

        assert_eq!(req.name, "my-image");
        assert_eq!(req.secret_uuid, None);
        assert_eq!(std::fs::read(&file_path).unwrap(), b"data");
        let _ = std::fs::remove_file(&file_path);
    }

    #[actix_rt::test]
    async fn reject_invalid_forms() {
        let file_path = temp_file_path("upload-invalid");
        let invalid_forms: [&[(&str, &[u8])]; 4] = [
            // the name is missing
            &[("image_type", b"disk"), ("file", b"data")],
            // the file is missing
            &[("name", b"my-image"), ("image_type", b"disk")],
            // only one file is allowed
            &[
                ("name", b"my-image"),
                ("image_type", b"disk"),
                ("file", b"data"),
                ("file", b"data"),
            ],
            // unknown fields are rejected
            &[
                ("name", b"my-image"),
                ("image_type", b"disk"),
                ("other", b"x"),
            ],
        ];

        for parts in invalid_forms {
            let result = read_form(multipart(parts), &file_path).await;
            assert!(matches!(result, Err(ErrorResponse::BadRequest(_))));
        }
        let _ = std::fs::remove_file(&file_path);
    }
}
