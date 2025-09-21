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
            <div class="card-label">Checkpoint</div>
            <div class="card-content">
                <!-- Add button -->
                <button class="add-button" @click="openAddModal">+</button>

                <table class="overview-table" v-if="checkpoints.length > 0">
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="checkpoint in checkpoints"
                            :key="checkpoint.uuid"
                        >
                            <td>{{ checkpoint.uuid }}</td>
                            <td>
                                <!-- Dropdown menu -->
                                <div
                                    class="table-dropdown"
                                    @click.stop="
                                        toggleDropdown(checkpoint.uuid)
                                    "
                                >
                                    ⋮
                                    <div
                                        v-if="openDropdown === checkpoint.uuid"
                                        class="table-dropdown-menu"
                                    >
                                        <button
                                            @click="openDeleteModal(checkpoint)"
                                        >
                                            Delete
                                        </button>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    </tbody>
                </table>

                <p v-else>No checkpoints found</p>
            </div>
        </div>

        <!-- Delete Confirmation Modal -->
        <div
            v-if="showDeleteModal"
            class="modal-overlay"
            @click.self="cancelDeleteModal"
        >
            <div class="modal checkpoint-delete-modal">
                <div class="modal-topbar">
                    <span>Delete checkpoint</span>
                </div>
                <div class="modal-content">
                    <p>Are you sure you want to delete?</p>
                    <strong>Checkpoint: {{ checkpointToDelete?.uuid }}</strong>
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

const checkpoints = ref<{ uuid: string; checkpointName: string }[]>([]);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const checkpointToDelete = ref<{ uuid: string; checkpointName: string } | null>(
    null,
);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchCheckpoints() {
    try {
        const token = localStorage.getItem("jwtToken");
        const response = await api.hanami_api.get("/v1alpha/checkpoint", {
            headers: { Authorization: `Bearer ${token}` },
        });
        checkpoints.value = response.data.checkpoints;
    } catch (err) {
        console.error("Failed to load checkpoints", err);
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
// Delete modal
//=============================================================================
function openDeleteModal(checkpoint: { uuid: string; checkpointName: string }) {
    checkpointToDelete.value = checkpoint;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    checkpointToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    if (!checkpointToDelete.value) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.hanami_api.delete(
            `/v1alpha/checkpoint/${checkpointToDelete.value.uuid}`,
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchCheckpoints();
    } catch (err) {
        console.error("Failed to delete checkpoint", err);
    } finally {
        cancelDeleteModal();
    }
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchCheckpoints);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.checkpoint-create-modal {
    height: 18rem;
    width: 20rem;
}

.checkpoint-delete-modal {
    height: 16rem;
    width: 20rem;
}
</style>
