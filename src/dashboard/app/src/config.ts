// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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

let config: AppConfig | null = null;

export async function loadConfig() {
    console.log("Loading config.json...");
    const res = await fetch("/config.json", { cache: "no-store" });
    console.log("Fetch result:", res);

    if (!res.ok) {
        throw new Error("Failed to load config.json");
    }

    config = await res.json();
    console.log("Config loaded:", config);
}

export function getConfig(): AppConfig {
    if (config === null) {
        throw new Error("Config not loaded yet");
    }
    return config;
}
