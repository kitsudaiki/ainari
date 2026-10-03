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
    <div class="card">
        <div class="card-label">VM Types</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="vmTypes.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Number of Cores</th>
                        <th>Amount of Memory</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="vmType in vmTypes" :key="vmType.uuid">
                        <td>{{ vmType.uuid }}</td>
                        <td>{{ vmType.name }}</td>
                        <td>{{ vmType.number_of_cores }}</td>
                        <td>{{ vmType.amount_of_memory }} MiB</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(vmType.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === vmType.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openInfoModal(vmType)">
                                        Info
                                    </button>
                                    <button @click="openEditModal(vmType)">
                                        Edit
                                    </button>
                                    <button @click="openDeleteModal(vmType)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <NoEntries v-else text="No vm-types found" />
        </div>

        <VmTypeCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <VmTypeEditModal
            v-if="showEditModal"
            :vm-type="vmTypeToEdit"
            :icons="icons"
            @accept="acceptEditModal"
            @cancel="cancelEditModal"
        />

        <VmTypeDeleteModal
            v-if="showDeleteModal"
            :vm-type="vmTypeToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />

        <VmTypeInfoModal
            v-if="showInfoModal"
            :vm-type="vmTypeToInfo"
            :icons="icons"
            @cancel="cancelInfoModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";

import { hanami } from "@/api";
import type { VmTypeBasicResp } from "@/api";

import VmTypeCreateModal from "./vm_type_create_modal.vue";
import VmTypeEditModal from "./vm_type_edit_modal.vue";
import VmTypeDeleteModal from "./vm_type_delete_modal.vue";
import VmTypeInfoModal from "./vm_type_info_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

const errorPopupMsg = ref<string>("");
const vmTypes = ref<VmTypeBasicResp[]>([]);
const showAddModal = ref(false);
const showEditModal = ref(false);
const showDeleteModal = ref(false);
const showInfoModal = ref(false);
const openDropdown = ref<string | null>(null);
const vmTypeToEdit = ref<VmTypeBasicResp | null>(null);
const vmTypeToDelete = ref<VmTypeBasicResp | null>(null);
const vmTypeToInfo = ref<VmTypeBasicResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchVmTypes() {
    try {
        vmTypes.value = await hanami.listVmTypes();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load vm-types");
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
// Add vm-type modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}

async function acceptAddModal() {
    await fetchVmTypes();
    cancelAddModal();
}

//=============================================================================
// Edit modal
//=============================================================================
function openEditModal(vmType: VmTypeBasicResp) {
    vmTypeToEdit.value = vmType;
    showEditModal.value = true;
    openDropdown.value = null;
}
function cancelEditModal() {
    showEditModal.value = false;
    vmTypeToEdit.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptEditModal() {
    await fetchVmTypes();
    cancelEditModal();
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(vmType: VmTypeBasicResp) {
    vmTypeToDelete.value = vmType;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    vmTypeToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchVmTypes();
    cancelDeleteModal();
}

//=============================================================================
// Info modal
//=============================================================================
function openInfoModal(vmType: VmTypeBasicResp) {
    vmTypeToInfo.value = vmType;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    vmTypeToInfo.value = null;
    openDropdown.value = null; // close any open action dropdown
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchVmTypes);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>
