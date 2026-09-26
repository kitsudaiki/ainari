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

import axios, { AxiosError } from "axios";

/**
 * Handles and formats Axios errors into user-friendly messages
 *
 * @param err - The AxiosError to handle
 * @param baseMessage - Base error message to use as attachment for the messages or as replacement when no specific message is available
 *
 * @returns Formatted error message string
 */
export function handleAxiosError(
    err: AxiosError,
    baseMessage = "An unexpected error occurred",
): string {
    // Check if the error is an AxiosError
    if (!axios.isAxiosError(err)) {
        return baseMessage;
    }

    // Handle response errors (when the request was made and the server responded)
    if (err.response) {
        const status = err.response.status;
        const message = responseMessage(err.response.data) ?? baseMessage;

        return `${baseMessage}: API error ${status}: ${message}`;
    }

    // Handle request errors (when the request was made but no response was received)
    if (err.request) {
        return `${baseMessage}: API did not respond`;
    }

    // Handle other errors (when the request was not made)
    return `${baseMessage}: ${err.message}`;
}

/**
 * Extracts the error message from the body of an error-response
 *
 * The backends send their error messages as plain text, so the body is a string. A JSON-body
 * with a `message`-field is supported as well.
 *
 * @param data - The body of the error-response
 *
 * @returns The error message, or undefined if the body contains none
 */
function responseMessage(data: unknown): string | undefined {
    if (typeof data === "string") {
        return data.trim() !== "" ? data.trim() : undefined;
    }
    if (data !== null && typeof data === "object" && "message" in data) {
        const message = (data as { message: unknown }).message;
        return typeof message === "string" ? message : undefined;
    }
    return undefined;
}
