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

export type AppConfig = {
    apiUrl: string;
};

// Private configuration storage
let config: AppConfig | null = null;

/**
 * Loads the application configuration from config.json
 *
 * @throws {Error} If the configuration file cannot be loaded
 */
export async function loadConfig(): Promise<void> {
    try {
        const response = await fetch("/config.json", { cache: "no-store" });
        if (!response.ok) {
            throw new Error(
                `Failed to load configuration: ${response.statusText}`,
            );
        }
        config = await response.json();
    } catch (error) {
        console.error("Error loading configuration:", error);
        throw error; // Re-throw to allow caller to handle
    }
}

/**
 * Retrieves the current application configuration
 *
 * @returns {AppConfig} The loaded configuration
 *
 * @throws {Error} If the configuration hasn't been loaded yet
 */
export function getConfig(): AppConfig {
    if (config === null) {
        throw new Error(
            "Configuration not loaded. Please call loadConfig() first.",
        );
    }
    return config;
}
