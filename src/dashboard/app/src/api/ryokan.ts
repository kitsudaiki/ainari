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
    HostBasicResp,
    HostResp,
    ImageBasicResp,
    ImageResp,
    ImageType,
} from "./types";

//=============================================================================
// image
//=============================================================================

/**
 * `POST /v1alpha/image/{type}/{name}`
 *
 * Uploads the files of a new image. The number of files depends on the type:
 * `csv` and `disk` expect a single file, `mnist` expects the image-file together
 * with the label-file.
 *
 * @param imageType - One of `csv`, `mnist` or `disk`
 * @param name - Name of the new image
 * @param files - Files to upload, in the order expected by the type
 */
export async function createImage(
    imageType: ImageType,
    name: string,
    files: File[],
): Promise<ImageResp> {
    const formData = new FormData();
    files.forEach((file, index) => {
        formData.append(`file${index + 1}`, file);
    });

    const resp = await ryokanClient().post(
        `/v1alpha/image/${imageType}/${name}`,
        formData,
        { headers: { "Content-Type": "multipart/form-data" } },
    );
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
export async function listOnsenHosts(): Promise<HostBasicResp[]> {
    const resp = await ryokanClient().get("/v1alpha/host/admin");
    return resp.data.hosts;
}

/** `GET /v1alpha/host/{host_uuid}/admin` */
export async function getOnsenHost(uuid: string): Promise<HostResp> {
    const resp = await ryokanClient().get(`/v1alpha/host/${uuid}/admin`);
    return resp.data;
}

/** `DELETE /v1alpha/host/{host_uuid}/admin` */
export async function deleteOnsenHost(uuid: string): Promise<void> {
    await ryokanClient().delete(`/v1alpha/host/${uuid}/admin`);
}
