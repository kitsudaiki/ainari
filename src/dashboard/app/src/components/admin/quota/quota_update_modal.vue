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
        <div class="modal quota-update-modal">
            <div class="modal-topbar">
                <span>Update quota of user: {{ quota?.user_id }}</span>
            </div>

            <div class="modal-content">
                <template v-for="field in fields" :key="field.key">
                    <div class="field-row">
                        <label :for="field.key">{{ field.label }}: </label>
                        <input
                            class="number-input"
                            :id="field.key"
                            v-model.number="values[field.key]"
                            type="number"
                            :min="0"
                            :class="{ invalid_input: errors[field.key] }"
                        />
                    </div>
                    <p v-if="errors[field.key]" class="error-msg">
                        Maximum quota must be a positive number
                    </p>
                    <br />
                </template>
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
import { ref, reactive } from "vue";

import { miko } from "@/api";
import type { QuotaBasicResp, QuotaSetReq } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    quota: QuotaBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");

// all fields of `QuotaSetReq`, so that adding a new quota-field on the server-side
// only needs one more entry here
const fields: { key: keyof QuotaSetReq; label: string }[] = [
    { key: "max_virtual_machine", label: "Maximum Virtual Machines" },
    { key: "max_image", label: "Maximum Images" },
    { key: "max_snapshot", label: "Maximum Snapshots" },
    { key: "max_secret", label: "Maximum Secrets" },
    { key: "max_network", label: "Maximum Networks" },
    { key: "max_floating_ip", label: "Maximum Floating IPs" },
    { key: "max_taskqueue", label: "Maximum Taskqueue" },
];

// the values are edited on a copy, so that a cancel leaves the table untouched
const values = reactive<QuotaSetReq>({
    max_virtual_machine: props.quota?.max_virtual_machine ?? 0,
    max_image: props.quota?.max_image ?? 0,
    max_snapshot: props.quota?.max_snapshot ?? 0,
    max_secret: props.quota?.max_secret ?? 0,
    max_network: props.quota?.max_network ?? 0,
    max_floating_ip: props.quota?.max_floating_ip ?? 0,
    max_taskqueue: props.quota?.max_taskqueue ?? 0,
});

const errors = reactive<Record<keyof QuotaSetReq, boolean>>({
    max_virtual_machine: false,
    max_image: false,
    max_snapshot: false,
    max_secret: false,
    max_network: false,
    max_floating_ip: false,
    max_taskqueue: false,
});

async function handleAccept() {
    if (!props.quota) return;

    let hasError = false;
    for (const field of fields) {
        errors[field.key] = values[field.key] < 0;
        hasError = hasError || errors[field.key];
    }

    if (hasError) {
        return;
    }

    try {
        await miko.setQuota(props.quota.user_id, { ...values });
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to update quota");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.quota-update-modal {
    width: 40rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}

.field-row {
    display: grid;
    grid-template-columns: 15rem 15rem; /* fixed input width */
    align-items: center;
    align-self: center;
}

.number-input {
    width: 5rem;
}
</style>
