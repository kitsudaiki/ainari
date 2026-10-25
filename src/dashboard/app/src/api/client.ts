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

/** Client for the ryokan, which handles images. */
export function ryokanClient(): AxiosInstance {
    return createClient(getAuthContext().ryokan_address);
}

/** Client for the omamori, which handles secrets and public-keys. */
export function omamoriClient(): AxiosInstance {
    return createClient(getAuthContext().omamori_address);
}

/** Client for the omikuji, which collects the error- and audit-logs of all components. */
export function omikujiClient(): AxiosInstance {
    return createClient(getAuthContext().omikuji_address);
}

/** Client for the isekai, which backs up and restores virtual-machines and projects. */
export function isekaiClient(): AxiosInstance {
    return createClient(getAuthContext().isekai_address);
}

/** True, if isekai is deployed, so the backups can be used. */
export function hasIsekai(): boolean {
    return !!getAuthContext().isekai_address;
}

/**
 * Creates a client for the sakura of a single virtual-machine. The sakura is not
 * reachable directly, but only through a proxy of the torii, which has its own
 * address for every virtual-machine.
 *
 * @param toriiAddress - Address of the proxy of the virtual-machine. It is
 *                       provided as `torii_address` by the get-endpoint and as
 *                       `proxy_address` by the list-endpoint of the virtual-machines.
 *
 * @returns Pre-configured axios-instance
 */
export function sakuraClient(toriiAddress: string): AxiosInstance {
    return createClient(toriiAddress);
}
