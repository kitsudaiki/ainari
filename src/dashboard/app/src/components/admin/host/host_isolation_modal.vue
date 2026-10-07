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
        <div class="modal host-isolation-modal">
            <div class="modal-topbar">
                <span>Set host isolation: {{ host?.name }}</span>
            </div>
            <div class="modal-content">
                <p>
                    An isolated host is only used by the virtual machines of a
                    single project. The isolation can only be changed, while no
                    virtual machine runs on the host.
                </p>
                <br />
                <div>
                    <label class="checkbox-label">
                        <input type="checkbox" v-model="isHostIsolated" />
                        Isolated host
                    </label>
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
import { ref } from "vue";

import { hanami } from "@/api";
import type { SakuraHostBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    host: SakuraHostBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

// the flag is edited on a copy, so that a cancel leaves the table untouched
const isHostIsolated = ref(props.host?.is_host_isolated ?? false);
const errorPopupMsg = ref<string>("");

async function handleAccept() {
    if (!props.host) return;

    // nothing to change
    if (isHostIsolated.value === props.host.is_host_isolated) {
        emit("cancel");
        return;
    }

    try {
        await hanami.setSakuraHostIsolation(props.host.uuid, {
            is_host_isolated: isHostIsolated.value,
        });
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to set host isolation",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.host-isolation-modal {
    width: 30rem;
}
</style>
