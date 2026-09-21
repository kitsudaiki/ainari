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
        <div class="modal virtual-machine-reserve-modal">
            <div class="modal-topbar">
                <span>Reserve virtual machine</span>
            </div>
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.name"
                        type="text"
                        placeholder="Name"
                        :class="{ invalid_input: nameError }"
                    />
                    <p v-if="nameError" class="error-msg">
                        Name must be at least 4 characters
                    </p>
                </div>
                <br />
                <div class="field-row">
                    <label for="numberOfCores">Number of cores: </label>
                    <input
                        class="number-input"
                        id="numberOfCores"
                        v-model.number="form.number_of_cores"
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
                    <label for="memorySize">Memory size (MiB): </label>
                    <input
                        class="number-input"
                        id="memorySize"
                        v-model.number="memorySizeMib"
                        type="number"
                        :min="1"
                        :class="{ invalid_input: memoryError }"
                    />
                </div>
                <p v-if="memoryError" class="error-msg">
                    Memory size must be at least 1 MiB
                </p>
                <br />
                <div class="field-row">
                    <label for="network">Network: </label>
                    <select
                        id="network"
                        v-model="form.network_uuid"
                        class="select-dropdown"
                        :class="{ invalid_input: networkError }"
                    >
                        <option value="" disabled>Select a network</option>
                        <option
                            v-for="network in networks"
                            :key="network.uuid"
                            :value="network.uuid"
                        >
                            {{ network.name }} ({{ network.subnet }})
                        </option>
                    </select>
                </div>
                <p v-if="networkError" class="error-msg">
                    A network must be selected
                </p>
                <p v-if="networks.length === 0" class="error-msg">
                    No networks available. Create one under Network first.
                </p>
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
import { ref, reactive, onMounted } from "vue";

import { hanami } from "@/api";
import type { NetworkBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const nameError = ref(false);
const coresError = ref(false);
const memoryError = ref(false);
const networkError = ref(false);

const networks = ref<NetworkBasicResp[]>([]);

// The api expects the memory-size in bytes, while the input is in MiB, because that
// is the unit a user actually wants to type in.
const memorySizeMib = ref(1024);

const form = reactive({
    name: "",
    number_of_cores: 1,
    network_uuid: "",
});

async function fetchNetworks() {
    try {
        networks.value = await hanami.listNetworks();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load networks");
    }
}

async function handleAccept() {
    nameError.value = form.name.length < 4;
    coresError.value = form.number_of_cores < 1;
    memoryError.value = memorySizeMib.value < 1;
    networkError.value = form.network_uuid === "";

    if (
        nameError.value ||
        coresError.value ||
        memoryError.value ||
        networkError.value
    ) {
        return;
    }

    try {
        await hanami.reserveVirtualMachine({
            name: form.name,
            number_of_cores: form.number_of_cores,
            memory_size: memorySizeMib.value * 1024 * 1024,
            network_uuid: form.network_uuid,
        });

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to reserve virtual machine",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchNetworks);
</script>

<style scoped>
.virtual-machine-reserve-modal {
    min-width: 30rem;
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
    width: 8rem;
}
</style>
