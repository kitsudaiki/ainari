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

                <table v-if="clusters.length > 0">
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

        <!-- Add Cluster Modal -->
        <div
            v-if="showAddModal"
            class="modal-overlay"
            @click.self="cancelAddModal"
        >
            <div class="modal cluster-create-modal">
                <!-- Modal topbar -->
                <div class="modal-topbar">
                    <span>Create cluster</span>
                </div>
                <div class="modal-content">
                    <input
                        v-model="newCluster.clusterName"
                        type="text"
                        placeholder="Cluster-Name"
                        required
                    />
                    <div>
                        <label>Cluster template:</label>
                        <textarea
                            id="template_input"
                            v-model="newCluster.clusterTemplate"
                            type="text"
                            required
                        ></textarea>
                    </div>
                </div>

                <div class="modal-bottombar">
                    <div class="modal-actions">
                        <button class="icon-button" @click="acceptAddModal">
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
            <div class="modal cluster-delete-modal">
                <div class="modal-topbar">
                    <span>Delete cluster</span>
                </div>
                <div class="modal-content">
                    <p>Are you sure you want to delete?</p>
                    <strong>Cluster: {{ clusterToDelete?.uuid }}</strong>
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

const clusters = ref<{ uuid: string; clusterName: string }[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const newCluster = ref({
    clusterTemplate: "",
    clusterName: "",
});
const passwordError = ref("");
const clusterToDelete = ref<{ uuid: string; clusterName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchClusters() {
    try {
        const token = localStorage.getItem("jwtToken");
        const response = await api.hanami_api.get("/v1alpha/cluster", {
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
    newCluster.value.clusterTemplate = "";
    newCluster.value.clusterName = "";
}

async function acceptAddModal() {
    try {
        const token = localStorage.getItem("jwtToken");
        await api.hanami_api.post(
            "/v1alpha/cluster",
            {
                name: newCluster.value.clusterName,
                template: newCluster.value.clusterTemplate,
            },
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchClusters();
        cancelAddModal();
    } catch (err) {
        passwordError.value = err;
        console.error("Failed to create cluster", err);
    }
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
    if (!clusterToDelete.value) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.hanami_api.delete(
            `/v1alpha/cluster/${clusterToDelete.value.uuid}`,
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchClusters();
    } catch (err) {
        console.error("Failed to delete cluster", err);
    } finally {
        cancelDeleteModal();
    }
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

<style scoped>
.cluster-create-modal {
    height: 35rem;
    width: 30rem;
}

#template_input {
    height: 18rem;
}

.cluster-delete-modal {
    height: 16rem;
    width: 20rem;
}
</style>
