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

import type { FilterDirection } from "@/api";

/** Include-list of a network filter, which a rule belongs to. */
export type FilterRuleType = "ip_range" | "port";

/** One entry of an include-list of the network filter of a virtual machine. */
export interface FilterRule {
    direction: FilterDirection;
    type: FilterRuleType;
    /** The entry in its canonical notation, like `10.0.0.0/24` or `22`. */
    spec: string;
}
