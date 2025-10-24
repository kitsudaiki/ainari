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
            <div class="card-label">Cluster</div>
            <div class="card-content">
                <!-- Add button -->
                <button class="add-button" @click="openAddModal">+</button>

                <table class="overview-table" v-if="clusters.length > 0">
                    <thead>
                        <tr>
                            <th>UUID</th>
                            <th>Name</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="cluster in clusters" :key="cluster.uuid">
                            <td>{{ cluster.uuid }}</td>
                            <td>{{ cluster.name }}</td>
                            <td>
                                <!-- Dropdown menu -->
                                <div
                                    class="table-dropdown"
                                    @click.stop="toggleDropdown(cluster.uuid)"
                                >
                                    ⋮
                                    <div
                                        v-if="openDropdown === cluster.uuid"
                                        class="table-dropdown-menu"
                                    >
                                        <button
                                            @click="switchToTasks(cluster.uuid)"
                                        >
                                            Show tasks
                                        </button>
                                        <button
                                            @click="openDeleteModal(cluster)"
                                        >
                                            Delete
                                        </button>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    </tbody>
                </table>

                <p v-else>No clusters found</p>
            </div>
        </div>

        <ClusterCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <ClusterDeleteModal
            v-if="showDeleteModal"
            :cluster="clusterToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import api from "../../../api";

import ClusterCreateModal from "./cluster_create_modal.vue";
import ClusterDeleteModal from "./cluster_delete_modal.vue";

const clusters = ref<{ uuid: string; clusterName: string }[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const passwordError = ref("");
const clusterToDelete = ref<{ uuid: string; clusterName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

const emit = defineEmits<{
    (e: "change-view", view: string, id: string): void;
}>();

function switchToTasks(cluster_uuid: string) {
    const view: string = "WorkloadTask";
    const id: string = cluster_uuid;
    emit("change-view", { view, id });
}

async function fetchClusters() {
    try {
        const token = localStorage.getItem("jwtToken");
        const response = await api.sakura_api.get("/v1alpha/cluster", {
            headers: { Authorization: `Bearer ${token}` },
        });
        clusters.value = response.data.clusters;
    } catch (err) {
        console.error("Failed to load clusters", err);
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
// Add cluster modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}

async function acceptAddModal() {
    await fetchClusters();
    cancelAddModal();
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(cluster: { uuid: string; clusterName: string }) {
    clusterToDelete.value = cluster;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    clusterToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}

async function acceptDeleteModal() {
    await fetchClusters();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchClusters);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>
