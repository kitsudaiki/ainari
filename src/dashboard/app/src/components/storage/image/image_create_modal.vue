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
        <div class="modal image-create-modal">
            <!-- Modal topbar -->
            <div class="modal-topbar">
                <span>Create image</span>
            </div>

            <!-- Modal content -->
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.imageName"
                        type="text"
                        placeholder="Image-Name"
                        :class="{ invalid_input: imageNameError }"
                    />
                    <p v-if="imageNameError" class="error-msg">
                        Image-Name must be at least 4 characters
                    </p>
                </div>
                <div class="field-row">
                    <label for="secret">Secret: </label>
                    <select id="secret" v-model="form.secretUuid" class="select-dropdown">
                        <option value="">None (generate new secret)</option>
                        <option
                            v-for="secret in secrets"
                            :key="secret.uuid"
                            :value="secret.uuid"
                        >
                            {{ secret.name }}
                        </option>
                    </select>
                </div>
                <p class="hint-msg">
                    The image is encrypted with the selected secret. Without a selection, a
                    new secret is generated for the image.
                </p>
                <div>
                    <div class="tab">
                        <button
                            class="tablinks"
                            :class="{ active: isSelected('disk') }"
                            @click="selectTab('disk')"
                        >
                            DISK
                        </button>
                    </div>
                    <div class="image-tabcontent">
                        <div v-show="selectedTab === 'disk'">
                            <br />
                            <label>
                                Boot-disk of the virtual machine:
                                <input type="file" @change="onFile1Change" />
                            </label>
                        </div>
                    </div>
                    <p v-if="fileError" class="error-msg">
                        All files of the selected type must be provided
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
import { ref, reactive, onMounted } from "vue";

import { omamori, ryokan } from "@/api";
import type { ImageType, SecretBasicResp } from "@/api";
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
const imageNameError = ref(false);
const fileError = ref(false);

const secrets = ref<SecretBasicResp[]>([]);

const form = reactive({
    imageName: "",
    secretUuid: "",
});

async function fetchSecrets() {
    try {
        secrets.value = await omamori.listSecrets();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load secrets");
    }
}
const file1 = ref<File | null>(null);

const onFile1Change = (event: Event) => {
    const target = event.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
        file1.value = target.files[0];
    }
};

/**
 * Collects the files, which the currently selected type expects. A disk-image is a
 * single file.
 *
 * @returns The files in the order expected by the endpoint, or null if one is missing
 */
function collectFiles(): File[] | null {
    if (!file1.value) return null;
    return [file1.value];
}

async function handleAccept() {
    imageNameError.value = form.imageName.length < 4;

    const files = collectFiles();
    fileError.value = files === null;

    if (imageNameError.value || files === null) {
        return;
    }

    try {
        await ryokan.createImage(
            selectedTab.value,
            form.imageName,
            files,
            form.secretUuid || undefined,
        );
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            `Failed to upload ${selectedTab.value.toUpperCase()}-file`,
        );
    }
}

function cancel() {
    emit("cancel");
}

//=============================================================================
// Tabs
//=============================================================================
const selectedTab = ref<ImageType>("disk");

function selectTab(tab: ImageType) {
    selectedTab.value = tab;
    // the files of the previous type do not fit the new one
    file1.value = null;
    fileError.value = false;
}

function isSelected(tab: ImageType) {
    return selectedTab.value === tab;
}

onMounted(fetchSecrets);
</script>

<style scoped>
.image-create-modal {
    width: 30rem;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

.field-row {
    display: grid;
    grid-template-columns: 6rem 22rem;
    align-items: center;
    margin-top: 1rem;
}

.image-tabcontent {
    margin-top: 0.5rem;
    height: 7rem;
}
</style>
