<!-- 
// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
                                    <button @click="openDeleteModal(dataset)">
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

        <DatasetCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <DatasetDeleteModal
            v-if="showDeleteModal"
            :dataset="datasetToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import axios from "axios";

import context from "../../../auth_context";
import DatasetCreateModal from "./dataset_create_modal.vue";
import DatasetDeleteModal from "./dataset_delete_modal.vue";

const datasets = ref<{ uuid: string; datasetName: string; email: string }[]>(
    [],
);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);

const passwordError = ref("");
const datasetToDelete = ref<{ uuid: string; datasetName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchDatasets() {
    try {
        const authContext = context.getAuthContext();
        const ryokan_api = axios.create({
            baseURL: authContext.ryokan_address,
        });

        const response = await ryokan_api.get("/v1alpha/dataset", {
            headers: { Authorization: `Bearer ${authContext.token}` },
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
}

async function acceptAddModal() {
    await fetchDatasets();
    cancelAddModal();
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(dataset: { id: string; datasetName: string }) {
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
    await fetchDatasets();
    cancelDeleteModal();
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
</script>
