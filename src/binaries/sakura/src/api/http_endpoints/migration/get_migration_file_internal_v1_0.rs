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

use std::path::Path as FilePath;

use actix_web::HttpResponse;
use actix_web::web::Path;
use apistos::api_operation;

use crate::core::virtual_machine::cloud_hypervisor::migration::transfer::compressed_file_stream;
use crate::core::virtual_machine::cloud_hypervisor::vm_socket_path;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::migration_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "migration",
    summary = "Get file of migration",
    description = r###"Stream a file of a virtual_machine, which is exported on this host for its migration.

The file is sent as a single zstd-frame, which contains the size and a checksum of the content.
The size of the uncompressed file is also sent in the header `X-Migration-File-Size`. The
virtual_machine has to be exported and its cloud-hypervisor process stopped, so the file doesn't
change during the transfer."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn get_migration_file_internal(
    path: Path<MigrationFilePath>,
    context: UserContext,
) -> Result<HttpResponse, ErrorResponse> {
    let virtual_machine_uuid = path.virtual_machine_uuid;
    let virtual_machine_data =
        super::get_exported_virtual_machine(&virtual_machine_uuid, &context)?;

    // the export stops the process, which would still write into the disk otherwise
    if FilePath::new(&vm_socket_path(&virtual_machine_uuid)).exists() {
        return Err(ErrorResponse::Conflict(format!(
            "Virtual_machine '{virtual_machine_uuid}' still has a running cloud-hypervisor process."
        )));
    }

    let file_path = match path.file {
        MigrationFile::RootDisk => virtual_machine_data.root_disk_path.ok_or_else(|| {
            ErrorResponse::Conflict(format!(
                "Virtual_machine '{virtual_machine_uuid}' has no root-disk."
            ))
        })?,
        MigrationFile::Seed => virtual_machine_data.seed_path,
    };

    let (size, stream) = compressed_file_stream(FilePath::new(&file_path))
        .map_err(|e| map_internal_error(&format!("stream {file_path}"), e))?;

    log::info!(
        "Send {} of virtual_machine '{virtual_machine_uuid}' with {size} bytes for its migration",
        path.file
    );

    Ok(HttpResponse::Ok()
        .content_type("application/zstd")
        .insert_header((MIGRATION_FILE_SIZE_HEADER, size.to_string()))
        .streaming(stream))
}
