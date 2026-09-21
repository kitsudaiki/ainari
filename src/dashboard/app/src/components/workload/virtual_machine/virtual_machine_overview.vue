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
        <div class="card-label">Virtual Machines</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openReserveModal">+</button>

            <table
                class="overview-table"
                v-if="virtualMachines.length > 0"
            >
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Address</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr
                        v-for="virtualMachine in virtualMachines"
                        :key="virtualMachine.uuid"
                    >
                        <td>{{ virtualMachine.uuid }}</td>
                        <td>{{ virtualMachine.name }}</td>
                        <td>
                            {{ torii_base_address }}:{{
                                virtualMachine.proxy_port
                            }}
                        </td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="
                                    toggleDropdown(virtualMachine.uuid)
                                "
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === virtualMachine.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button
                                        @click="openInfoModal(virtualMachine)"
                                    >
                                        Info
                                    </button>
                                    <button
                                        @click="openCreateModal(virtualMachine)"
                                    >
                                        Deploy
                                    </button>
                                    <button
                                        @click="
                                            switchToTasks(virtualMachine.uuid)
                                        "
                                    >
                                        Show tasks
                                    </button>
                                    <button
                                        @click="openDeleteModal(virtualMachine)"
                                    >
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No virtual machines found</p>
        </div>

        <VirtualMachineReserveModal
            v-if="showReserveModal"
            :icons="icons"
            @accept="acceptReserveModal"
            @cancel="cancelReserveModal"
        />

        <VirtualMachineCreateModal
            v-if="showCreateModal"
            :virtual_machine="virtualMachineToCreate"
            :icons="icons"
            @accept="acceptCreateModal"
            @cancel="cancelCreateModal"
        />

        <VirtualMachineInfoModal
            v-if="showInfoModal"
            :virtual_machine="virtualMachineToShow"
            :icons="icons"
            @cancel="cancelInfoModal"
        />

        <VirtualMachineDeleteModal
            v-if="showDeleteModal"
            :virtual_machine="virtualMachineToDelete"
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

import { getAuthContext } from "@/auth_context";
import { hanami } from "@/api";
import type { VirtualMachineBasicResp } from "@/api";
import VirtualMachineReserveModal from "./virtual_machine_reserve_modal.vue";
import VirtualMachineCreateModal from "./virtual_machine_create_modal.vue";
import VirtualMachineInfoModal from "./virtual_machine_info_modal.vue";
import VirtualMachineDeleteModal from "./virtual_machine_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const virtualMachines = ref<VirtualMachineBasicResp[]>([]);
const torii_base_address = ref<string | null>("");
const showReserveModal = ref(false);
const showCreateModal = ref(false);
const showInfoModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const virtualMachineToCreate = ref<VirtualMachineBasicResp | null>(null);
const virtualMachineToShow = ref<VirtualMachineBasicResp | null>(null);
const virtualMachineToDelete = ref<VirtualMachineBasicResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

const emit = defineEmits<{
    (e: "change-view", payload: { view: string; id: string }): void;
}>();

function switchToTasks(virtual_machine_uuid: string) {
    emit("change-view", { view: "WorkloadTask", id: virtual_machine_uuid });
}

async function fetchVirtualMachines() {
    try {
        torii_base_address.value = getAuthContext().torii_base_address;
        virtualMachines.value = await hanami.listVirtualMachines();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load virtual machines",
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
// Reserve modal
//=============================================================================
function openReserveModal() {
    showReserveModal.value = true;
}
function cancelReserveModal() {
    showReserveModal.value = false;
}
async function acceptReserveModal() {
    await fetchVirtualMachines();
    cancelReserveModal();
}

//=============================================================================
// Create modal, which deploys image and public-key on a reserved machine
//=============================================================================
function openCreateModal(virtualMachine: VirtualMachineBasicResp) {
    virtualMachineToCreate.value = virtualMachine;
    showCreateModal.value = true;
    openDropdown.value = null;
}
function cancelCreateModal() {
    showCreateModal.value = false;
    virtualMachineToCreate.value = null;
    openDropdown.value = null;
}
async function acceptCreateModal(virtual_machine_uuid: string) {
    cancelCreateModal();
    switchToTasks(virtual_machine_uuid);
}

//=============================================================================
// Info modal
//=============================================================================
function openInfoModal(virtualMachine: VirtualMachineBasicResp) {
    virtualMachineToShow.value = virtualMachine;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    virtualMachineToShow.value = null;
    openDropdown.value = null;
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(virtualMachine: VirtualMachineBasicResp) {
    virtualMachineToDelete.value = virtualMachine;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    virtualMachineToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchVirtualMachines();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchVirtualMachines);

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
    width: 30%;
}
</style>
