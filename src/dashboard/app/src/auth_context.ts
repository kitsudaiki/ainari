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
 *
 * @property {string | null} token - Current authentication token
 * @property {number | null} expire_timestamp - Unix timestamp when the token expires
 * @property {string | null} is_admin - "true" if the user is admin, otherwise "false". This is a string instead of a boolean to not depend on one single bit for the check.
 * @property {string | null} miko_address - Address for the Miko service
 * @property {string | null} hanami_address - Address for the Hanami service
 * @property {string | null} ryokan_address - Address for the Ryokan service
 * @property {string | null} torii_address - Full address for the Torii service (including port)
 * @property {string | null} torii_base_address - Base address for the Torii service (without port)
 * @property {string | null} omamori_address - Address for the Omamori service
 */
export interface AuthContext {
    token: string | null;
    expire_timestamp: number | null;
    is_admin: string | null;
    miko_address: string | null;
    hanami_address: string | null;
    ryokan_address: string | null;
    torii_address: string | null;
    torii_base_address: string | null;
    omamori_address: string | null;
}

/**
 * Extracts the admin status from a JWT token
 *
 * @param {string} token - The JWT token to parse
 *
 * @returns {string | null} "true" if the user is admin, "false" otherwise, or null if parsing fails
 *
 * @description
 * This function parses the JWT token to extract the is_admin field from the payload.
 * The JWT is split into its components, and the payload is base64 decoded and parsed.
 * If any step fails, it returns "false" as a fallback.
 */
export function getIsAdminFromJwt(token: string): string | null {
    try {
        // JWT format: header.payload.signature
        const payloadBase64 = token.split(".")[1];
        if (!payloadBase64) return "false";

        // Convert Base64URL to standard Base64 by replacing URL-safe characters
        const payloadJson = atob(
            payloadBase64.replace(/-/g, "+").replace(/_/g, "/"),
        );

        const payload = JSON.parse(payloadJson);
        return payload.is_admin ?? "false";
    } catch {
        return "false";
    }
}

/**
 * Extracts the expiration timestamp from a JWT token
 *
 * @param {string} token - The JWT token to parse
 *
 * @returns {number | null} The expiration timestamp in seconds, or null if parsing fails
 *
 * @description
 * This function parses the JWT token to extract the exp field (expiration time)
 * from the payload. The expiration time is typically in seconds since Unix epoch.
 * If any step fails, it returns 0 as a fallback.
 */
export function getExpireTimesamp(token: string): number | null {
    try {
        // JWT format: header.payload.signature
        const payloadBase64 = token.split(".")[1];
        if (!payloadBase64) return 0;

        // Convert Base64URL to standard Base64 by replacing URL-safe characters
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
 * @param {string} token - The authentication token to be stored
 *
 * @description
 * This function creates an Axios instance to fetch service endpoints from the API.
 * It extracts admin status and expiration time from the JWT token, then stores
 * all this information in the browser's localStorage under the key "ainari_authContext".
 * The torii_base_address is derived from the torii_address by removing the port number.
 */
export async function createAuthContext(token: string) {
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
    if (authContext.torii_address) {
        const toriiParts = authContext.torii_address.split(":");
        if (toriiParts.length > 1) {
            // Remove the port and any path/query parameters
            authContext.torii_base_address = toriiParts[0];
        } else {
            // If there's no port, use the full address as the base
            authContext.torii_base_address = authContext.torii_address;
        }
    } else {
        // Handle the case where torii_address is null
        authContext.torii_base_address = null;
    }

    // Store the complete auth context in localStorage as a JSON string
    localStorage.setItem("ainari_authContext", JSON.stringify(authContext));
}

/**
 * Retrieves the authentication context from localStorage
 *
 * @returns {AuthContext} The authentication context object with all service addresses and token
 *          Returns null values for all properties if nothing is stored
 *
 * @description
 * This function retrieves the stored authentication context from localStorage.
 * If no data is found, it returns an AuthContext object with all properties set to null.
 * If data is found, it parses the JSON and merges it with a default AuthContext object.
 */
export function getAuthContext(): AuthContext {
    const stored = localStorage.getItem("ainari_authContext");

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
