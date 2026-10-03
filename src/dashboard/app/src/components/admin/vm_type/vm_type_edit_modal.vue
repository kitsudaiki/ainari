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
        <div class="modal vm-type-edit-modal">
            <div class="modal-topbar">
                <span>Edit vm-type: {{ vmType?.name }}</span>
            </div>

            <div class="modal-content">
                <div>
                    <div class="field-row">
                        <label for="vm-type-name">Name: </label>
                        <input
                            id="vm-type-name"
                            v-model="form.name"
                            type="text"
                            :class="{ invalid_input: nameError }"
                        />
                    </div>
                    <p v-if="nameError" class="error-msg">
                        Name must be between 4 and 127 characters
                    </p>
                    <br />
                    <div class="field-row">
                        <label for="vm-type-cores">Number of Cores: </label>
                        <input
                            id="vm-type-cores"
                            class="number-input"
                            v-model.number="form.numberOfCores"
                            type="number"
                            :min="1"
                            :class="{ invalid_input: coresError }"
                        />
                    </div>
                    <p v-if="coresError" class="error-msg">
                        Number of cores must be at least 1
                    </p>
                    <br />
                    <div class="field-row">
                        <label for="vm-type-memory">Memory (MiB): </label>
                        <input
                            id="vm-type-memory"
                            class="number-input"
                            v-model.number="form.amountOfMemory"
                            type="number"
                            :min="1"
                            :class="{ invalid_input: memoryError }"
                        />
                    </div>
                    <p v-if="memoryError" class="error-msg">
                        Amount of memory must be at least 1 MiB
                    </p>
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
import { ref, reactive } from "vue";

import { hanami } from "@/api";
import type { VmTypeBasicResp, VmTypeUpdateReq } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    vmType: VmTypeBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

// the values are edited on a copy, so that a cancel leaves the table untouched
const form = reactive({
    name: props.vmType?.name ?? "",
    numberOfCores: props.vmType?.number_of_cores ?? 1,
    amountOfMemory: props.vmType?.amount_of_memory ?? 1,
});

const errorPopupMsg = ref<string>("");
const nameError = ref(false);
const coresError = ref(false);
const memoryError = ref(false);

async function handleAccept() {
    if (!props.vmType) return;

    nameError.value = form.name.length < 4 || form.name.length > 127;
    coresError.value =
        !Number.isInteger(form.numberOfCores) || form.numberOfCores < 1;
    memoryError.value =
        !Number.isInteger(form.amountOfMemory) || form.amountOfMemory < 1;

    if (nameError.value || coresError.value || memoryError.value) {
        return;
    }

    // only the changed values are sent, the backend keeps all others
    const body: VmTypeUpdateReq = {};
    if (form.name !== props.vmType.name) {
        body.name = form.name;
    }
    if (form.numberOfCores !== props.vmType.number_of_cores) {
        body.number_of_cores = form.numberOfCores;
    }
    if (form.amountOfMemory !== props.vmType.amount_of_memory) {
        body.amount_of_memory = form.amountOfMemory;
    }

    try {
        await hanami.updateVmType(props.vmType.uuid, body);
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to update vm-type");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.vm-type-edit-modal {
    width: 35rem;
}

.field-row {
    display: grid;
    grid-template-columns: 15rem 15rem;
    align-items: center;
}

.number-input {
    width: 7rem;
}
</style>
