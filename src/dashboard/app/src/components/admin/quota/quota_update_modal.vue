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
        <div class="modal quota-create-modal">
            <div class="modal-topbar">
                <span>Update quota of user: {{ quota.user_id }}</span>
            </div>

            <div class="modal-content">
                <div class="field-row">
                    <label for="maxModel">Maximum Model: </label>
                    <input
                        class="number-input"
                        id="maxModel"
                        v-model.number="quota.max_model"
                        type="number"
                        :min="0"
                        :class="{ invalid_input: quotaModelError }"
                    />
                </div>
                <p v-if="quotaModelError" class="error-msg">
                    Minimum quota must be a positive number
                </p>
                <br />
                <div class="field-row">
                    <label for="maxDataset">Maximum Datasets: </label>
                    <input
                        class="number-input"
                        id="maxDataset"
                        v-model.number="quota.max_dataset"
                        type="number"
                        :min="1"
                        :class="{ invalid_input: quotaDatasetError }"
                    />
                </div>
                <p v-if="quotaDatasetError" class="error-msg">
                    Maximum quota must be a positive number
                </p>
                <br />
                <div class="field-row">
                    <label for="maxCheckpoint">Maximum Checkpoints: </label>
                    <input
                        class="number-input"
                        id="maxCheckpoint"
                        v-model.number="quota.max_checkpoint"
                        type="number"
                        :min="0"
                        :class="{ invalid_input: quotaCheckpointError }"
                    />
                </div>
                <p v-if="quotaCheckpointError" class="error-msg">
                    Maximum quota must be a positive number
                </p>
                <br />
                <div class="field-row">
                    <label for="maxSecret">Maximum Secrets: </label>
                    <input
                        class="number-input"
                        id="maxSecret"
                        v-model.number="quota.max_secret"
                        type="number"
                        :min="0"
                        :class="{ invalid_input: quotaSecretError }"
                    />
                </div>
                <p v-if="quotaSecretError" class="error-msg">
                    Maximum quota must be a positive number
                </p>
                <br />
                <div class="field-row">
                    <label for="maxTaskqueue">Maximum Taskqueue: </label>
                    <input
                        class="number-input"
                        id="maxTaskqueue"
                        v-model.number="quota.max_taskqueue"
                        type="number"
                        :min="0"
                        :class="{ invalid_input: quotaTaskqueueError }"
                    />
                </div>
                <p v-if="quotaTaskqueueError" class="error-msg">
                    Maximum quota must be a positive number
                </p>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button class="icon-button" @click="handleAccept(quota)">
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
import axios from "axios";

import { getAuthContext } from "@/auth_context";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    quota: {
        user_id: string;
        max_model: number;
        max_dataset: number;
        max_checkpoint: number;
        max_secret: number;
        max_taskqueue: number;
    } | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const quotaModelError = ref(false);
const quotaDatasetError = ref(false);
const quotaCheckpointError = ref(false);
const quotaSecretError = ref(false);
const quotaTaskqueueError = ref(false);

async function handleAccept(quota: {
    user_id: string;
    max_model: number;
    max_dataset: number;
    max_checkpoint: number;
    max_secret: number;
    max_taskqueue: number;
}) {
    quotaModelError.value = quota.max_model < 0;
    quotaDatasetError.value = quota.max_dataset < 0;
    quotaCheckpointError.value = quota.max_checkpoint < 0;
    quotaSecretError.value = quota.max_secret < 0;
    quotaTaskqueueError.value = quota.max_taskqueue < 0;

    if (
        quotaModelError.value ||
        quotaDatasetError.value ||
        quotaCheckpointError.value ||
        quotaSecretError.value ||
        quotaTaskqueueError.value
    ) {
        return;
    }

    try {
        const authContext = getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        await miko_api.put(
            `/v1alpha/quota/${quota.user_id}/admin`,
            {
                max_model: quota.max_model,
                max_dataset: quota.max_dataset,
                max_checkpoint: quota.max_checkpoint,
                max_secret: quota.max_secret,
                max_taskqueue: quota.max_taskqueue,
            },
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );

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
.quota-create-modal {
    width: 40rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}

.number-form {
    display: flex;
    flex-direction: column;
    align-items: center; /* centers the whole block */
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
