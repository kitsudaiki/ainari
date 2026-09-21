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

// Single entry-point for the api-layer. The modules are namespaced by the component
// they talk to, because the same resource-name can exist on more than one component
// (for example `host` on the hanami and on the ryokan).

export * as hanami from "./hanami";
export * as miko from "./miko";
export * as omamori from "./omamori";
export * as ryokan from "./ryokan";
export * as sakura from "./sakura";
export * from "./types";
