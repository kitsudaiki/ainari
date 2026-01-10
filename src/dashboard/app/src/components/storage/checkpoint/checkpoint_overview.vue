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
        <div class="card-label">Checkpoint</div>
        <div class="card-content">
            <table class="overview-table" v-if="checkpoints.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr
                        v-for="checkpoint in checkpoints"
                        :key="checkpoint.uuid"
                    >
                        <td>{{ checkpoint.uuid }}</td>
                        <td>{{ checkpoint.name }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(checkpoint.uuid)"
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

        <CheckpointDeleteModal
            v-if="showDeleteModal"
            :checkpoint="checkpointToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import axios from "axios";

import context from "../../../auth_context";

import CheckpointDeleteModal from "./checkpoint_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const checkpoints = ref<{ uuid: string; checkpointName: string }[]>([]);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const checkpointToDelete = ref<{ uuid: string; checkpointName: string } | null>(
    null,
);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchCheckpoints() {
    try {
        const authContext = context.getAuthContext();
        const ryokan_api = axios.create({
            baseURL: authContext.ryokan_address,
        });

        const response = await ryokan_api.get("/v1alpha/checkpoint", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        checkpoints.value = response.data.checkpoints;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load checkpoints",
        );
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
function openDeleteModal(checkpoint: { id: string; checkpointName: string }) {
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
    await fetchCheckpoints();
    cancelDeleteModal();
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
