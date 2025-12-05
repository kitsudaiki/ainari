<!-- 
// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
            <div class="modal-topbar">
                <span>Create dataset</span>
            </div>
            <div class="modal-content">
                <input
                    v-model="form.datasetName"
                    type="text"
                    placeholder="Dataset-Name"
                    required
                />
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
                        <label>
                            <input type="file" @change="onFile2Change" />
                        </label>
                    </div>
                    <div v-show="selectedTab === 'mnist'">
                        <label>
                            <input type="file" @change="onFile1Change" />
                            <input type="file" @change="onFile2Change" />
                        </label>
                    </div>
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
</template>

<script lang="ts" setup>
import { ref, reactive } from "vue";
import axios from "axios";

import context from "../../../auth_context";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

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
    if (selectedTab.value === "mnist") {
        if (!file1.value || !file2.value) return;

        const formData = new FormData();
        formData.append("file1", file1.value);
        formData.append("file2", file2.value);

        try {
            const authContext = context.getAuthContext();
            const ryokan_api = axios.create({
                baseURL: authContext.ryokan_address,
            })
            
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
            // console.log("Upload success!", response.data);
        } catch (err) {
            console.error("Upload MNIST-file failed!", err);
        }
    }

    if (selectedTab.value === "csv") {
        if (!file1.value) return;

        const formData = new FormData();
        formData.append("file1", file1.value);

        try {
            const authContext = context.getAuthContext();
            const ryokan_api = axios.create({
                baseURL: authContext.ryokan_address,
            })
            
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
            // console.log("Upload success!", response.data);
        } catch (err) {
            console.error("Upload CSV-file failed!", err);
        }
    }

    console.log("Submitting form:", form);
    emit("accept");
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
    height: 26rem;
    width: 30rem;
}

.dataset-tabcontent {
    margin-top: 0.5rem;
    height: 7rem;
}
</style>
