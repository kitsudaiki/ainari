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
                <span>Deploy virtual machine</span>
            </div>
            <div class="modal-content">
                <p>
                    Installs image and public-key on the reserved machine and
                    boots it. This is queued as a task.
                </p>
                <strong>
                    Virtual machine: {{ virtual_machine?.name }}
                </strong>
                <br /><br />
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
                    An image must be selected
                </p>
                <p v-if="images.length === 0" class="error-msg">
                    No images available. Upload one under Storage first.
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
                    A public-key must be selected
                </p>
                <p v-if="publicKeys.length === 0" class="error-msg">
                    No public-keys available. Upload one under Security first.
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
import { ref, onMounted } from "vue";

import { hanami, omamori, ryokan, sakura } from "@/api";
import type {
    ImageBasicResp,
    PublicKeyBasicResp,
    VirtualMachineBasicResp,
} from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    virtual_machine: VirtualMachineBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept", virtual_machine_uuid: string): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const imageError = ref(false);
const publicKeyError = ref(false);

const images = ref<ImageBasicResp[]>([]);
const publicKeys = ref<PublicKeyBasicResp[]>([]);
const selectedImageUuid = ref<string>("");
const selectedPublicKeyUuid = ref<string>("");

async function fetchSelectableResources() {
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
    if (!props.virtual_machine) return;

    imageError.value = selectedImageUuid.value === "";
    publicKeyError.value = selectedPublicKeyUuid.value === "";

    if (imageError.value || publicKeyError.value) {
        return;
    }

    try {
        // the sakura is only reachable through the torii, so the port of the
        // virtual-machine has to be resolved first
        const virtualMachine = await hanami.getVirtualMachine(
            props.virtual_machine.uuid,
        );

        await sakura.createVirtualMachine(virtualMachine.torii_port, {
            vm_uuid: props.virtual_machine.uuid,
            image_uuid: selectedImageUuid.value,
            public_key_uuid: selectedPublicKeyUuid.value,
        });

        emit("accept", props.virtual_machine.uuid);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to deploy virtual machine",
        );
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
    grid-template-columns: 10rem 20rem; /* fixed input width */
    align-items: center;
}
</style>
