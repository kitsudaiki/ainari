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
        <div class="modal virtual-machine-create-modal">
            <div class="modal-topbar">
                <span>Create virtual machine</span>
            </div>
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.name"
                        type="text"
                        placeholder="Name"
                        :disabled="reservedVirtualMachine !== null"
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
                        :disabled="reservedVirtualMachine !== null"
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
                        :disabled="reservedVirtualMachine !== null"
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
                        :disabled="reservedVirtualMachine !== null"
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
                    <template v-if="networks.length === 0">
                        No networks available. Create one under Network first.
                    </template>
                    <template v-else>A network must be selected</template>
                </p>
                <br />
                <div class="field-row">
                    <label for="image">Image: </label>
                    <select
                        id="image"
                        v-model="selectedImageUuid"
                        class="select-dropdown"
                        :class="{ invalid_input: imageError }"
                    >
                        <option value="" disabled>Select an image</option>
                        <option
                            v-for="image in images"
                            :key="image.uuid"
                            :value="image.uuid"
                        >
                            {{ image.name }}
                        </option>
                    </select>
                </div>
                <p v-if="imageError" class="error-msg">
                    <template v-if="images.length === 0">
                        No images available. Upload one under Storage first.
                    </template>
                    <template v-else>An image must be selected</template>
                </p>
                <br />
                <div class="field-row">
                    <label for="publicKey">Public-key: </label>
                    <select
                        id="publicKey"
                        v-model="selectedPublicKeyUuid"
                        class="select-dropdown"
                        :class="{ invalid_input: publicKeyError }"
                    >
                        <option value="" disabled>Select a public-key</option>
                        <option
                            v-for="publicKey in publicKeys"
                            :key="publicKey.uuid"
                            :value="publicKey.uuid"
                        >
                            {{ publicKey.name }} ({{ publicKey.fingerprint }})
                        </option>
                    </select>
                </div>
                <p v-if="publicKeyError" class="error-msg">
                    <template v-if="publicKeys.length === 0">
                        No public-keys available. Upload one under Security
                        first.
                    </template>
                    <template v-else>A public-key must be selected</template>
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

import { hanami, omamori, ryokan, sakura } from "@/api";
import type {
    ImageBasicResp,
    NetworkBasicResp,
    PublicKeyBasicResp,
    VirtualMachineResp,
} from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept", virtual_machine_uuid: string): void;
    (e: "reserved"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const nameError = ref(false);
const coresError = ref(false);
const memoryError = ref(false);
const networkError = ref(false);
const imageError = ref(false);
const publicKeyError = ref(false);

const networks = ref<NetworkBasicResp[]>([]);
const images = ref<ImageBasicResp[]>([]);
const publicKeys = ref<PublicKeyBasicResp[]>([]);
const selectedImageUuid = ref<string>("");
const selectedPublicKeyUuid = ref<string>("");

// The api expects the memory-size in bytes, while the input is in MiB, because that
// is the unit a user actually wants to type in.
const memorySizeMib = ref(1024);

const form = reactive({
    name: "",
    number_of_cores: 1,
    network_uuid: "",
});

// Set as soon as the reserve-call was successful. If the following create-call fails,
// a retry only repeats the create-call instead of reserving another virtual machine.
const reservedVirtualMachine = ref<VirtualMachineResp | null>(null);

async function fetchSelectableResources() {
    try {
        networks.value = await hanami.listNetworks();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load networks");
    }

    try {
        images.value = await ryokan.listImages();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load images");
    }

    try {
        publicKeys.value = await omamori.listPublicKeys();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load public-keys",
        );
    }
}

async function handleAccept() {
    nameError.value = form.name.length < 4;
    coresError.value = form.number_of_cores < 1;
    memoryError.value = memorySizeMib.value < 1;
    networkError.value = form.network_uuid === "";
    imageError.value = selectedImageUuid.value === "";
    publicKeyError.value = selectedPublicKeyUuid.value === "";

    if (
        nameError.value ||
        coresError.value ||
        memoryError.value ||
        networkError.value ||
        imageError.value ||
        publicKeyError.value
    ) {
        return;
    }

    // reserve the virtual machine on one of the sakura-hosts
    if (reservedVirtualMachine.value === null) {
        try {
            reservedVirtualMachine.value = await hanami.reserveVirtualMachine({
                name: form.name,
                number_of_cores: form.number_of_cores,
                memory_size: memorySizeMib.value * 1024 * 1024,
                network_uuid: form.network_uuid,
            });
            emit("reserved");
        } catch (err) {
            errorPopupMsg.value = handleAxiosError(
                err,
                "Failed to reserve virtual machine",
            );
            return;
        }
    }

    // install image and public-key in the reserved virtual machine and boot it
    const virtualMachine = reservedVirtualMachine.value;
    if (virtualMachine === null) return;
    try {
        await sakura.createVirtualMachine(virtualMachine.torii_port, {
            vm_uuid: virtualMachine.uuid,
            image_uuid: selectedImageUuid.value,
            public_key_uuid: selectedPublicKeyUuid.value,
        });

        emit("accept", virtualMachine.uuid);
    } catch (err) {
        errorPopupMsg.value =
            handleAxiosError(err, "Failed to deploy virtual machine") +
            ` (virtual machine '${virtualMachine.uuid}' is still reserved` +
            " and has to be deleted manually, or retry the deployment)";
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchSelectableResources);
</script>

<style scoped>
.virtual-machine-create-modal {
    min-width: 34rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}

.field-row {
    display: grid;
    grid-template-columns: 15rem 20rem; /* fixed input width */
    align-items: center;
}

.number-input {
    width: 8rem;
}
</style>
