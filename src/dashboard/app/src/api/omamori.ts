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

// Endpoints of the omamori, mirroring `src/binaries/omamori/src/api/routes/v1alpha.rs`.

import { omamoriClient } from "./client";
import type {
    PublicKeyBasicResp,
    PublicKeyResp,
    SecretBasicResp,
    SecretResp,
} from "./types";

//=============================================================================
// secret
//=============================================================================

/** `POST /v1alpha/secret` - stores a secret, which is provided by the user. */
export async function createSecret(
    name: string,
    secretPayload: string,
): Promise<SecretResp> {
    const resp = await omamoriClient().post("/v1alpha/secret", {
        name,
        secret_payload: secretPayload,
    });
    return resp.data;
}

/** `POST /v1alpha/secret/generate` - lets the omamori generate the payload. */
export async function generateSecret(name: string): Promise<SecretResp> {
    const resp = await omamoriClient().post("/v1alpha/secret/generate", { name });
    return resp.data;
}

/** `GET /v1alpha/secret` */
export async function listSecrets(): Promise<SecretBasicResp[]> {
    const resp = await omamoriClient().get("/v1alpha/secret");
    return resp.data.secrets;
}

/** `GET /v1alpha/secret/{secret_uuid}` */
export async function getSecret(uuid: string): Promise<SecretResp> {
    const resp = await omamoriClient().get(`/v1alpha/secret/${uuid}`);
    return resp.data;
}

/** `GET /v1alpha/secret/{secret_uuid}/payload` */
export async function getSecretPayload(uuid: string): Promise<string> {
    const resp = await omamoriClient().get(`/v1alpha/secret/${uuid}/payload`);
    return resp.data.secret_payload;
}

/** `DELETE /v1alpha/secret/{secret_uuid}` */
export async function deleteSecret(uuid: string): Promise<void> {
    await omamoriClient().delete(`/v1alpha/secret/${uuid}`);
}

/** `GET /v1alpha/secret/count` */
export async function getSecretCount(): Promise<number> {
    const resp = await omamoriClient().get("/v1alpha/secret/count");
    return resp.data.number_of_items;
}

//=============================================================================
// public-key
//=============================================================================

/**
 * `POST /v1alpha/public_key`
 *
 * @param name - Name of the new entry
 * @param publicKey - The ssh-public-key in its one-line openssh-representation. The
 *                    fingerprint is calculated by the omamori and not provided here.
 */
export async function uploadPublicKey(
    name: string,
    publicKey: string,
): Promise<PublicKeyResp> {
    const resp = await omamoriClient().post("/v1alpha/public_key", {
        name,
        public_key: publicKey,
    });
    return resp.data;
}

/** `GET /v1alpha/public_key` */
export async function listPublicKeys(): Promise<PublicKeyBasicResp[]> {
    const resp = await omamoriClient().get("/v1alpha/public_key");
    return resp.data.public_keys;
}

/** `GET /v1alpha/public_key/{public_key_uuid}` */
export async function getPublicKey(uuid: string): Promise<PublicKeyResp> {
    const resp = await omamoriClient().get(`/v1alpha/public_key/${uuid}`);
    return resp.data;
}

/** `DELETE /v1alpha/public_key/{public_key_uuid}` */
export async function deletePublicKey(uuid: string): Promise<void> {
    await omamoriClient().delete(`/v1alpha/public_key/${uuid}`);
}
