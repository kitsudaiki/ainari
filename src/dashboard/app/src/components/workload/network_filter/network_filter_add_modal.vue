<!--
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
-->

<template>
    <div class="modal-overlay" @click.self="cancel">
        <div class="modal network-filter-add-modal">
            <div class="modal-topbar">
                <span>Add filter rules</span>
            </div>
            <div class="modal-content">
                <div class="field-row">
                    <label for="direction">Direction: </label>
                    <select
                        id="direction"
                        v-model="form.direction"
                        class="select-dropdown"
                    >
                        <option value="ingress">Ingress</option>
                        <option value="egress">Egress</option>
                    </select>
                </div>
                <p class="hint-msg">
                    Ingress filters the traffic towards the virtual machine by
                    its source, egress the traffic of the virtual machine by its
                    destination.
                </p>
                <br />
                <div class="field-row">
                    <label for="rule_type">Type: </label>
                    <select
                        id="rule_type"
                        v-model="form.type"
                        class="select-dropdown"
                    >
                        <option value="ip_range">IP range</option>
                        <option value="port">Port</option>
                    </select>
                </div>
                <br />
                <div>
                    <input
                        v-model="form.rules"
                        type="text"
                        :placeholder="placeholder"
                        :class="{ invalid_input: rulesError }"
                    />
                    <p v-if="rulesError" class="error-msg">
                        At least one rule is required
                    </p>
                    <p class="hint-msg">{{ hint }}</p>
                </div>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button class="icon-button" @click="handleAccept">
                        <img :src="icons.acceptIcon" alt="Accept" />
                    </button>
                    <button class="icon-button" @click="cancel">
                        <img :src="icons.cancelIcon" alt="Cancel" />
                    </button>
                </div>
            </div>
        </div>
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script lang="ts" setup>
import { ref, reactive, computed } from "vue";

import { hanami } from "@/api";
import type { FilterDirection } from "@/api";
import type { FilterRuleType } from "./network_filter_rule";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    virtual_machine_uuid: string | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const rulesError = ref(false);

const form = reactive<{
    direction: FilterDirection;
    type: FilterRuleType;
    rules: string;
}>({
    direction: "ingress",
    type: "ip_range",
    rules: "",
});

const placeholder = computed(() =>
    form.type === "ip_range"
        ? "IP ranges, like 10.0.0.0/24, 10.0.1.5"
        : "Ports, like 22, 8000-8100",
);

const hint = computed(() =>
    form.type === "ip_range"
        ? "Single addresses, subnets in CIDR notation or ranges FIRST-LAST, " +
          "separated by commas or spaces. As soon as a direction has an IP " +
          "range, only the listed addresses are allowed."
        : "Single ports or ranges FIRST-LAST, separated by commas or spaces. " +
          "As soon as a direction has a port, TCP and UDP are only allowed " +
          "on the listed ports.",
);

/** Splits the input into its rules. The backend validates each of them. */
function parseRules(input: string): string[] {
    return input
        .split(/[\s,]+/)
        .map((rule) => rule.trim())
        .filter((rule) => rule !== "");
}

async function handleAccept() {
    if (!props.virtual_machine_uuid) return;

    const rules = parseRules(form.rules);
    rulesError.value = rules.length === 0;
    if (rulesError.value) {
        return;
    }

    try {
        if (form.type === "ip_range") {
            await hanami.addNetworkFilterIpRanges(
                props.virtual_machine_uuid,
                form.direction,
                { ranges: rules },
            );
        } else {
            await hanami.addNetworkFilterPorts(
                props.virtual_machine_uuid,
                form.direction,
                { ports: rules },
            );
        }

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to add filter rules",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.network-filter-add-modal {
    width: 32rem;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

.field-row {
    display: grid;
    grid-template-columns: 10rem 18rem; /* fixed input width */
    align-items: center;
}
</style>
