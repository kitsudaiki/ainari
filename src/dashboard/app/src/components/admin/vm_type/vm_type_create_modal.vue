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
        <div class="modal vm-type-create-modal">
            <!-- Modal topbar -->
            <div class="modal-topbar">
                <span>Create vm-type</span>
            </div>

            <!-- Modal content -->
            <div class="modal-content">
                <div>
                    <div>
                        <input
                            v-model="form.name"
                            type="text"
                            placeholder="Name"
                            :class="{ invalid_input: nameError }"
                        />
                        <p v-if="nameError" class="error-msg">
                            Name must be between 4 and 127 characters
                        </p>
                    </div>
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

            <!-- Modal bottombar -->
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
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    name: "",
    numberOfCores: 1,
    amountOfMemory: 1024,
});

const errorPopupMsg = ref<string>("");
const nameError = ref(false);
const coresError = ref(false);
const memoryError = ref(false);

async function handleAccept() {
    nameError.value = form.name.length < 4 || form.name.length > 127;
    coresError.value =
        !Number.isInteger(form.numberOfCores) || form.numberOfCores < 1;
    memoryError.value =
        !Number.isInteger(form.amountOfMemory) || form.amountOfMemory < 1;

    if (nameError.value || coresError.value || memoryError.value) {
        return;
    }
    try {
        await hanami.createVmType({
            name: form.name,
            number_of_cores: form.numberOfCores,
            amount_of_memory: form.amountOfMemory,
        });

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to create vm-type");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.vm-type-create-modal {
    width: 30em;
    margin-bottom: 5rem;
}

.field-row {
    display: grid;
    grid-template-columns: 15rem 10rem;
    align-items: center;
}

.number-input {
    width: 7rem;
}
</style>
