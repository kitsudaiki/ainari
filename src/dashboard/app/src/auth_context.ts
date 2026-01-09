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

import axios from "axios";

import { getConfig } from "./config";

/**
 * Interface representing the authentication context
 * containing various service addresses and the auth token
 */
interface AuthContext {
    token: string | null; // Current authentication token
    expire_timestamp: number | null; // unix-timestamp, when the token expire
    is_admin: string | null; // true, if the user is admin
    miko_address: string | null; // Address for the Miko service
    hanami_address: string | null; // Address for the Hanami service
    ryokan_address: string | null; // Address for the Ryokan service
    torii_address: string | null; // Full address for the Torii service
    torii_base_address: string | null; // Base address for the Torii service (without port)
    omamori_address: string | null; // Address for the Omamori service
}

function getIsAdminFromJwt(token: string): string | null {
    try {
        // JWT format: header.payload.signature
        const payloadBase64 = token.split(".")[1];
        if (!payloadBase64) return "false";

        // Base64URL → Base64
        const payloadJson = atob(
            payloadBase64.replace(/-/g, "+").replace(/_/g, "/"),
        );

        const payload = JSON.parse(payloadJson);
        return payload.is_admin ?? "false";
    } catch {
        return "false";
    }
}

function getExpireTimesamp(token: string): number | null {
    try {
        // JWT format: header.payload.signature
        const payloadBase64 = token.split(".")[1];
        if (!payloadBase64) return 0;

        // Base64URL → Base64
        const payloadJson = atob(
            payloadBase64.replace(/-/g, "+").replace(/_/g, "/"),
        );

        const payload = JSON.parse(payloadJson);
        return payload.exp ?? 0;
    } catch {
        return 0;
    }
}

/**
 * Sets the authentication context in localStorage
 * This function fetches service endpoints and stores them along with the auth token
 *
 * @param token - The authentication token to be stored
 */
async function createAuthContext(token: string) {
    const { apiUrl } = getConfig();
    const miko_api = axios.create({
        baseURL: apiUrl,
    });

    const is_admin = getIsAdminFromJwt(token);
    const expire_timestamp = getExpireTimesamp(token);

    // Fetch the service endpoints from the API
    const endpoint_resp = await miko_api.get("/v1alpha/endpoints");

    // Initialize the auth context with the provided token
    // and environment variables or fetched service addresses
    const authContext: AuthContext = {
        token: token,
        expire_timestamp: expire_timestamp,
        is_admin: is_admin,
        miko_address: apiUrl, // Get Miko address from environment
        hanami_address: endpoint_resp.data.hanami.public_address,
        ryokan_address: endpoint_resp.data.ryokan.public_address,
        torii_address: endpoint_resp.data.torii.public_address,
        torii_base_address: null, // This will be set below
        omamori_address: endpoint_resp.data.omamori.public_address,
    };

    // Extract the base address from the Torii address (remove port number)
    // This is useful for constructing URLs without specifying a port
    const [part1, part2] = authContext.torii_address.split(":");
    authContext.torii_base_address = `${part1}:${part2}`;

    // Store the complete auth context in localStorage as a JSON string
    localStorage.setItem("ainari_authContext", JSON.stringify(authContext));
}

/**
 * Retrieves the authentication context from localStorage
 *
 * @returns The authentication context object with all service addresses and token
 *          Returns null values for all properties if nothing is stored
 */
function getAuthContext(): AuthContext {
    // Get the stored auth context from localStorage
    const stored = localStorage.getItem("ainari_authContext");
    console.log("get stored: ", stored);

    // Initialize with default null values and merge with stored values if they exist
    const authContext: AuthContext = {
        token: null,
        expire_timestamp: null,
        is_admin: null,
        miko_address: null,
        hanami_address: null,
        ryokan_address: null,
        torii_address: null,
        torii_base_address: null,
        omamori_address: null,
        ...(stored ? JSON.parse(stored) : {}), // Spread the parsed stored values
    };

    return authContext;
}

/**
 * Authentication module providing functions to set and get auth context
 */
export default {
    createAuthContext,
    getAuthContext,
    getIsAdminFromJwt,
    getExpireTimesamp,
};
