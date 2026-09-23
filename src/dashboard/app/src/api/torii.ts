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

// Endpoints of the torii, mirroring `src/binaries/torii/src/api/routes/v1alpha.rs`.
//
// Only the proxy-entries are covered here. Everything else on the torii (routes,
// network-filter, network-crypto and the network-interfaces) is registered as
// `/internal` and is only used by the other components, not by a user-token.

import { toriiClient } from "./client";
import type { ProxyBasicResp, ProxyResp } from "./types";

//=============================================================================
// proxy
//=============================================================================

/**
 * `GET /v1alpha/proxy`
 *
 * Lists the proxy-entries of the user. Every virtual-machine gets one entry, which
 * maps a port on the torii to the sakura of that virtual-machine.
 *
 * The entries are created and deleted by the hanami together with the
 * virtual-machine itself, so there is no create- or delete-function here. Both
 * endpoints exist only as `/internal`.
 */
export async function listProxies(): Promise<ProxyBasicResp[]> {
    const resp = await toriiClient().get("/v1alpha/proxy");
    // the field is named `proxys` in `ProxyListResp` of the backend
    return resp.data.proxys;
}

/** `GET /v1alpha/proxy/{proxy_uuid}` */
export async function getProxy(uuid: string): Promise<ProxyResp> {
    const resp = await toriiClient().get(`/v1alpha/proxy/${uuid}`);
    return resp.data;
}
