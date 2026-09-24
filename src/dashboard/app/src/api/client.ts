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

import axios, { AxiosInstance } from "axios";

import { getAuthContext } from "@/auth_context";

/**
 * Creates an axios-instance for the given base-address with the token of the
 * current auth-context already attached.
 *
 * @param baseURL - Base-address of the component to talk to
 *
 * @returns Pre-configured axios-instance
 */
function createClient(baseURL: string | null): AxiosInstance {
    const authContext = getAuthContext();

    return axios.create({
        baseURL: baseURL ?? undefined,
        headers: { Authorization: `Bearer ${authContext.token}` },
    });
}

/** Client for the miko, which handles authentication, users, projects and quotas. */
export function mikoClient(): AxiosInstance {
    return createClient(getAuthContext().miko_address);
}

/** Client for the hanami, which handles virtual-machines, networks and floating-ips. */
export function hanamiClient(): AxiosInstance {
    return createClient(getAuthContext().hanami_address);
}

/** Client for the ryokan, which handles images and snapshots. */
export function ryokanClient(): AxiosInstance {
    return createClient(getAuthContext().ryokan_address);
}

/** Client for the omamori, which handles secrets and public-keys. */
export function omamoriClient(): AxiosInstance {
    return createClient(getAuthContext().omamori_address);
}

/** Client for the torii, which handles the proxy-entries. */
export function toriiClient(): AxiosInstance {
    return createClient(getAuthContext().torii_address);
}

/**
 * Creates a client for the sakura of a single virtual-machine. The sakura is not
 * reachable directly, but only through the torii, which provides one port per
 * virtual-machine.
 *
 * @param toriiPort - Port, which the torii assigned to the virtual-machine. It is
 *                    provided as `torii_port` by the get-endpoint and as `proxy_port`
 *                    by the list-endpoint of the virtual-machines.
 *
 * @returns Pre-configured axios-instance
 */
export function sakuraClient(toriiPort: number): AxiosInstance {
    const authContext = getAuthContext();

    return createClient(`${authContext.torii_base_address}:${toriiPort}`);
}
