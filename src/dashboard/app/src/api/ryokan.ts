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

// Endpoints of the ryokan, mirroring `src/binaries/ryokan/src/api/routes/v1alpha.rs`.

import { ryokanClient } from "./client";
import type {
    HostBackupTargetUpdateReq,
    OnsenHostBasicResp,
    OnsenHostResp,
    ImageBasicResp,
    ImageResp,
    ImageType,
} from "./types";

//=============================================================================
// image
//=============================================================================

/** Options of an upload. */
export interface UploadOptions {
    /** Called while the files are sent, with the bytes already sent and the total bytes. */
    onProgress?: (loaded: number, total: number) => void;
    /** Aborts the upload, if the signal is triggered. */
    signal?: AbortSignal;
}

/**
 * `POST /v1alpha/image`
 *
 * Uploads a new image as multipart-form. The number of files depends on the type:
 * `disk` expects a single file. Type and size of the file are identified by ryokan.
 *
 * @param imageType - Type of the image, currently only `disk`
 * @param name - Name of the new image
 * @param files - Files to upload, in the order expected by the type
 * @param secretUuid - Optional existing secret to encrypt the image with. Without it, a new
 *                     secret is generated for the image.
 * @param options - Optional callback for the upload-progress and signal to abort the upload
 */
export async function createImage(
    imageType: ImageType,
    name: string,
    files: File[],
    secretUuid?: string,
    options: UploadOptions = {},
): Promise<ImageResp> {
    const formData = new FormData();
    formData.append("name", name);
    formData.append("image_type", imageType);
    if (secretUuid) {
        formData.append("secret_uuid", secretUuid);
    }
    files.forEach((file) => {
        formData.append("file", file);
    });

    const resp = await ryokanClient().post("/v1alpha/image", formData, {
        headers: { "Content-Type": "multipart/form-data" },
        signal: options.signal,
        onUploadProgress: (event) => {
            // the total is only unknown, if the browser can not determine the size of the body
            if (options.onProgress && event.total) {
                options.onProgress(event.loaded, event.total);
            }
        },
    });
    return resp.data;
}

/** `GET /v1alpha/image` */
export async function listImages(): Promise<ImageBasicResp[]> {
    const resp = await ryokanClient().get("/v1alpha/image");
    return resp.data.images;
}

/** `GET /v1alpha/image/{image_uuid}` */
export async function getImage(uuid: string): Promise<ImageResp> {
    const resp = await ryokanClient().get(`/v1alpha/image/${uuid}`);
    return resp.data;
}

/** `DELETE /v1alpha/image/{image_uuid}` */
export async function deleteImage(uuid: string): Promise<void> {
    await ryokanClient().delete(`/v1alpha/image/${uuid}`);
}

/** `GET /v1alpha/image/count` */
export async function getImageCount(): Promise<number> {
    const resp = await ryokanClient().get("/v1alpha/image/count");
    return resp.data.number_of_items;
}

//=============================================================================
// onsen-host
//=============================================================================

/** `GET /v1alpha/host/admin` */
export async function listOnsenHosts(): Promise<OnsenHostBasicResp[]> {
    const resp = await ryokanClient().get("/v1alpha/host/admin");
    return resp.data.hosts;
}

/** `GET /v1alpha/host/{host_uuid}/admin` */
export async function getOnsenHost(uuid: string): Promise<OnsenHostResp> {
    const resp = await ryokanClient().get(`/v1alpha/host/${uuid}/admin`);
    return resp.data;
}

/**
 * `PUT /v1alpha/host/{host_uuid}/backup_target/admin`
 *
 * A backup-target only stores the backups of isekai. It can only be marked, while it stores no
 * images or snapshots, and only be unmarked, while it stores no backups.
 */
export async function setOnsenHostBackupTarget(
    uuid: string,
    body: HostBackupTargetUpdateReq,
): Promise<OnsenHostResp> {
    const resp = await ryokanClient().put(`/v1alpha/host/${uuid}/backup_target/admin`, body);
    return resp.data;
}

/** `DELETE /v1alpha/host/{host_uuid}/admin` */
export async function deleteOnsenHost(uuid: string): Promise<void> {
    await ryokanClient().delete(`/v1alpha/host/${uuid}/admin`);
}
