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
        <div class="modal dataset-create-modal">
            <!-- Modal topbar -->
            <div class="modal-topbar">
                <span>Create dataset</span>
            </div>

            <!-- Modal content -->
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.datasetName"
                        type="text"
                        placeholder="Dataset-Name"
                        :class="{ invalid_input: datasetNameError }"
                    />
                    <p v-if="datasetNameError" class="error-msg">
                        Dataset-Name must be at least 4 characters
                    </p>
                </div>
                <div>
                    <div class="tab">
                        <button
                            class="tablinks"
                            :class="{ active: isSelected('csv') }"
                            @click="selectTab('csv')"
                        >
                            CSV
                        </button>
                        <button
                            class="tablinks"
                            :class="{ active: isSelected('mnist') }"
                            @click="selectTab('mnist')"
                        >
                            MNIST
                        </button>
                    </div>
                    <div class="dataset-tabcontent">
                        <div v-show="selectedTab === 'csv'">
                            <br />
                            <label>
                                <input type="file" @change="onFile2Change" />
                            </label>
                        </div>
                        <div v-show="selectedTab === 'mnist'">
                            <br />
                            <label>
                                <input type="file" @change="onFile1Change" />
                                <input type="file" @change="onFile2Change" />
                            </label>
                        </div>
                    </div>
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
import axios from "axios";

import { getAuthContext } from "@/auth_context";
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
const datasetNameError = ref(false);

const form = reactive({
    datasetName: "",
});
const file1 = ref<File | null>(null);
const file2 = ref<File | null>(null);

const onFile1Change = (event: Event) => {
    const target = event.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
        file1.value = target.files[0];
    }
};

const onFile2Change = (event: Event) => {
    const target = event.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
        file2.value = target.files[0];
    }
};

async function handleAccept() {
    datasetNameError.value = form.datasetName.length < 4;

    if (datasetNameError.value) {
        return;
    }
    if (selectedTab.value === "mnist") {
        if (!file1.value || !file2.value) return;

        const formData = new FormData();
        formData.append("file1", file1.value);
        formData.append("file2", file2.value);

        try {
            const authContext = getAuthContext();
            const ryokan_api = axios.create({
                baseURL: authContext.ryokan_address,
            });

            const response = await ryokan_api.post(
                `/v1alpha/dataset/mnist/${form.datasetName}`,
                formData,
                {
                    headers: {
                        "Content-Type": "multipart/form-data",
                        Authorization: `Bearer ${authContext.token}`,
                    },
                },
            );

            emit("accept");
        } catch (err) {
            errorPopupMsg.value = handleAxiosError(
                err,
                "Failed to upload MNIST-file",
            );
        }
    }

    if (selectedTab.value === "csv") {
        if (!file1.value) return;

        const formData = new FormData();
        formData.append("file1", file1.value);

        try {
            const authContext = getAuthContext();
            const ryokan_api = axios.create({
                baseURL: authContext.ryokan_address,
            });

            const response = ryokan_api.post(
                `/v1alpha/dataset/csv/${form.datasetName}`,
                formData,
                {
                    headers: {
                        "Content-Type": "multipart/form-data",
                        Authorization: `Bearer ${authContext.token}`,
                    },
                },
            );

            emit("accept");
        } catch (err) {
            errorPopupMsg.value = handleAxiosError(
                err,
                "Failed to upload CSV-file",
            );
        }
    }
}

function cancel() {
    emit("cancel");
}

//=============================================================================
// Tabs
//=============================================================================
const selectedTab = ref<"csv" | "mnist">("csv");

function selectTab(tab: "csv" | "mnist") {
    selectedTab.value = tab;
}

function isSelected(tab: "csv" | "mnist") {
    return selectedTab.value === tab;
}
</script>

<style scoped>
.dataset-create-modal {
    width: 30rem;
}

.dataset-tabcontent {
    margin-top: 0.5rem;
    height: 7rem;
}
/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
