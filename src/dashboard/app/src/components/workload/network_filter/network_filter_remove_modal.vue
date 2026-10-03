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
        <div class="modal network-filter-remove-modal">
            <div class="modal-topbar">
                <span>Remove filter rule</span>
            </div>
            <div class="modal-content">
                <p>Are you sure you want to remove this rule?</p>
                <strong>{{ description }}</strong>
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
import { ref, computed } from "vue";

import { hanami } from "@/api";
import type { FilterRule } from "./network_filter_rule";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    virtual_machine_uuid: string | null;
    rule: FilterRule | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();
const errorPopupMsg = ref<string>("");

const description = computed(() => {
    if (!props.rule) return "";
    const direction = props.rule.direction === "ingress" ? "Ingress" : "Egress";
    const type = props.rule.type === "ip_range" ? "IP range" : "Port";
    return `${direction}: ${type} ${props.rule.spec}`;
});

async function handleAccept() {
    if (!props.rule || !props.virtual_machine_uuid) return;
    try {
        // removing the last rule of a direction removes its whole filter, which
        // allows all traffic of this direction again
        if (props.rule.type === "ip_range") {
            await hanami.deleteNetworkFilterIpRanges(
                props.virtual_machine_uuid,
                props.rule.direction,
                { ranges: [props.rule.spec] },
            );
        } else {
            await hanami.deleteNetworkFilterPorts(
                props.virtual_machine_uuid,
                props.rule.direction,
                { ports: [props.rule.spec] },
            );
        }

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to remove filter rule",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.network-filter-remove-modal {
    width: 30rem;
}
</style>
