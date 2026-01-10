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

/**
 * Formats a date-time string into a standardized format (YYYY-MM-DD HH:MM:SS).
 * Handles various date string formats by cleaning them first to ensure JavaScript Date compatibility.
 *
 * @param input - The date string to format. Expected formats include ISO 8601 and other common date formats.
 *
 * @returns A formatted date string in YYYY-MM-DD HH:MM:SS format.
 *
 * @throws Error if no date string is provided or if the input string cannot be parsed as a valid date.
 */
function formatDateTime(input?: string): string {
    // Validate input: ensure a date string was provided
    if (!input) {
        throw new Error("No date string provided");
    }

    // Clean the input string by removing nanoseconds (fractional seconds)
    // This makes the string compatible with JavaScript's Date constructor
    // Example: "2023-05-15T12:30:45.123456789Z" becomes "2023-05-15T12:30:45Z"
    const cleaned = input.replace(/\.\d+/, "");

    // Create a Date object from the cleaned string
    const date = new Date(cleaned);

    // Verify the Date object was created successfully
    if (isNaN(date.getTime())) {
        throw new Error("Invalid date string");
    }

    // Extract date components with proper zero-padding
    const yyyy = date.getFullYear(); // 4-digit year (e.g., 2023)
    const mm = String(date.getMonth() + 1).padStart(2, "0"); // Month (1-12), zero-padded
    const dd = String(date.getDate()).padStart(2, "0"); // Day of month (1-31), zero-padded
    const hh = String(date.getHours()).padStart(2, "0"); // Hours (0-23), zero-padded
    const mi = String(date.getMinutes()).padStart(2, "0"); // Minutes (0-59), zero-padded
    const ss = String(date.getSeconds()).padStart(2, "0"); // Seconds (0-59), zero-padded

    // Combine components into the final formatted string
    return `${yyyy}-${mm}-${dd} ${hh}:${mi}:${ss}`;
}

export default {
    formatDateTime,
};
