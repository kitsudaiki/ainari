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
        <div class="card-label">Secrets</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="secrets.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="secret in secrets" :key="secret.uuid">
                        <td>{{ secret.uuid }}</td>
                        <td>{{ secret.name }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(secret.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === secret.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openPayloadModal(secret)">
                                        Show payload
                                    </button>
                                    <button @click="openDeleteModal(secret)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No secrets found</p>
        </div>

        <SecretCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <SecretPayloadModal
            v-if="showPayloadModal"
            :secret="secretToShow"
            :icons="icons"
            @cancel="cancelPayloadModal"
        />

        <SecretDeleteModal
            v-if="showDeleteModal"
            :secret="secretToDelete"
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

import { omamori } from "@/api";
import type { SecretBasicResp } from "@/api";
import SecretCreateModal from "./secret_create_modal.vue";
import SecretPayloadModal from "./secret_payload_modal.vue";
import SecretDeleteModal from "./secret_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const secrets = ref<SecretBasicResp[]>([]);
const showAddModal = ref(false);
const showPayloadModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const secretToShow = ref<SecretBasicResp | null>(null);
const secretToDelete = ref<SecretBasicResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchSecrets() {
    try {
        secrets.value = await omamori.listSecrets();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load secrets");
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
// Add secret modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}
async function acceptAddModal() {
    await fetchSecrets();
    cancelAddModal();
}

//=============================================================================
// Payload modal
//=============================================================================
function openPayloadModal(secret: SecretBasicResp) {
    secretToShow.value = secret;
    showPayloadModal.value = true;
    openDropdown.value = null;
}
function cancelPayloadModal() {
    showPayloadModal.value = false;
    secretToShow.value = null;
    openDropdown.value = null;
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(secret: SecretBasicResp) {
    secretToDelete.value = secret;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    secretToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchSecrets();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchSecrets);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
/* Columns 2 through n-1 share remaining space equally */
th:not(:first-child):not(:last-child),
td:not(:first-child):not(:last-child) {
    width: 40%;
}
</style>
