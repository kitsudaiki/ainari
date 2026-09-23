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
        <div class="modal secret-payload-modal">
            <div class="modal-topbar">
                <span>Payload of {{ secret?.name }}</span>
            </div>
            <div class="modal-content">
                <p v-if="!revealed">
                    The payload is only requested from the omamori when it is
                    actually shown.
                </p>
                <button v-if="!revealed" @click="reveal">Show payload</button>

                <pre v-else class="payload">{{ payload }}</pre>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
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

import { omamori } from "@/api";
import type { SecretBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    secret: SecretBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const payload = ref<string>("");
const revealed = ref(false);

async function reveal() {
    if (!props.secret) return;

    try {
        payload.value = await omamori.getSecretPayload(props.secret.uuid);
        revealed.value = true;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load secret-payload",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.secret-payload-modal {
    width: 34rem;
}

.payload {
    font-family: monospace;
    word-break: break-all;
    white-space: pre-wrap;
}
</style>
