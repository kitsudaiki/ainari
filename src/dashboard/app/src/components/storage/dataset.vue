<!-- 
// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//         http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License. 
-->

<template>
    <div class="overview">
        <div class="card">
            <div class="card-label">Dataset</div>
            <div class="card-content">
                <!-- Add button -->
                <button class="add-button" @click="openAddModal">+</button>

                <table class="overview-table" v-if="datasets.length > 0">
                    <thead>
                        <tr>
                            <th>UUID</th>
                            <th>Name</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="dataset in datasets" :key="dataset.uuid">
                            <td>{{ dataset.uuid }}</td>
                            <td>{{ dataset.name }}</td>
                            <td>
                                <!-- Dropdown menu -->
                                <div
                                    class="table-dropdown"
                                    @click.stop="toggleDropdown(dataset.uuid)"
                                >
                                    ⋮
                                    <div
                                        v-if="openDropdown === dataset.uuid"
                                        class="table-dropdown-menu"
                                    >
                                        <button
                                            @click="openDeleteModal(dataset)"
                                        >
                                            Delete
                                        </button>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    </tbody>
                </table>

                <p v-else>No datasets found</p>
            </div>
        </div>

        <!-- Add Dataset Modal -->
        <div
            v-if="showAddModal"
            class="modal-overlay"
            @click.self="cancelAddModal"
        >
            <div class="modal dataset-create-modal">
                <!-- Modal topbar -->
                <div class="modal-topbar">
                    <span>Create dataset</span>
                </div>

                <div class="modal-content">
                    <input
                        v-model="newDataset.datasetName"
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
                        <button
                            class="icon-button"
                            @click="acceptAddModal"
                            :disabled="!file1 || !file2"
                        >
                            <img :src="icons.acceptIcon" alt="Accept" />
                        </button>
                        <button class="icon-button" @click="cancelAddModal">
                            <img :src="icons.cancelIcon" alt="Cancel" />
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Delete Confirmation Modal -->
        <div
            v-if="showDeleteModal"
            class="modal-overlay"
            @click.self="cancelDeleteModal"
        >
            <div class="modal dataset-delete-modal">
                <div class="modal-topbar">
                    <span>Delete dataset</span>
                </div>
                <div class="modal-content">
                    <p>Are you sure you want to delete?</p>
                    <strong>Dataset: {{ datasetToDelete?.uuid }}</strong>
                </div>

                <div class="modal-bottombar">
                    <div class="modal-actions">
                        <button class="icon-button" @click="acceptDeleteModal">
                            <img :src="icons.acceptIcon" alt="Accept" />
                        </button>
                        <button class="icon-button" @click="cancelDeleteModal">
                            <img :src="icons.cancelIcon" alt="Cancel" />
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import api from "../../api";

const datasets = ref<{ uuid: string; datasetName: string; email: string }[]>(
    [],
);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const newDataset = ref({
    datasetName: "",
});
const file1 = ref<File | null>(null);
const file2 = ref<File | null>(null);
const passwordError = ref("");
const datasetToDelete = ref<{ uuid: string; datasetName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchDatasets() {
    try {
        const token = localStorage.getItem("jwtToken");
        const response = await api.hanami_api.get("/v1alpha/dataset", {
            headers: { Authorization: `Bearer ${token}` },
        });
        datasets.value = response.data.datasets;
    } catch (err) {
        console.error("Failed to load datasets", err);
    }
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(uuid: string) {
    openDropdown.value = openDropdown.value === uuid ? null : uuid;
}

function handleClickOutside(event: MouseEvent) {
    const dropdowns = document.querySelectorAll(".table-dropdown");
    let clickedInside = false;
    dropdowns.forEach((dropdown) => {
        if (dropdown.contains(event.target as Node)) {
            clickedInside = true;
        }
    });
    if (!clickedInside) {
        openDropdown.value = null; // close the dropdown
    }
}

//=============================================================================
// Add dataset modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
    newDataset.value.datasetName = "";
    passwordError.value = "";
}

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

async function acceptAddModal() {
    if (selectedTab.value === "mnist") {
        if (!file1.value || !file2.value) return;

        const formData = new FormData();
        formData.append("file1", file1.value);
        formData.append("file2", file2.value);

        try {
            const token = localStorage.getItem("jwtToken");
            const response = await api.hanami_api.post(
                `/v1alpha/dataset/mnist/${newDataset.value.datasetName}`,
                formData,
                {
                    headers: {
                        "Content-Type": "multipart/form-data",
                        Authorization: `Bearer ${token}`,
                    },
                },
            );
            // console.log("Upload success!", response.data);
            await fetchDatasets();
            cancelAddModal();
        } catch (err) {
            console.error("Upload MNIST-file failed!", err);
        }
    }

    if (selectedTab.value === "csv") {
        if (!file1.value) return;

        const formData = new FormData();
        formData.append("file1", file1.value);

        try {
            const token = localStorage.getItem("jwtToken");
            const response = await api.hanami_api.post(
                `/v1alpha/dataset/csv/${newDataset.value.datasetName}`,
                formData,
                {
                    headers: {
                        "Content-Type": "multipart/form-data",
                        Authorization: `Bearer ${token}`,
                    },
                },
            );
            // console.log("Upload success!", response.data);
            await fetchDatasets();
            cancelAddModal();
        } catch (err) {
            console.error("Upload CSV-file failed!", err);
        }
    }
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(dataset: { uuid: string; datasetName: string }) {
    datasetToDelete.value = dataset;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    datasetToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    if (!datasetToDelete.value) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.hanami_api.delete(
            `/v1alpha/dataset/${datasetToDelete.value.uuid}`,
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchDatasets();
        cancelDeleteModal();
    } catch (err) {
        console.error("Failed to delete dataset", err);
    }
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchDatasets);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});

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
//=============================================================================
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

.dataset-delete-modal {
    height: 16rem;
    width: 20rem;
}
</style>
