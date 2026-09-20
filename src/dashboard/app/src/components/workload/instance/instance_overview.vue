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
        <div class="card-label">Instances</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="instances.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Adress</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="instance in instances" :key="instance.uuid">
                        <td>{{ instance.uuid }}</td>
                        <td>{{ instance.name }}</td>
                        <td>{{torii_base_address}}:{{ instance.proxy_port }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(instance.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === instance.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button
                                        @click="switchToTasks(instance.uuid)"
                                    >
                                        Show tasks
                                    </button>
                                    <button @click="openDeleteModal(instance)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No instances found</p>
        </div>

        <InstanceCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <InstanceDeleteModal
            v-if="showDeleteModal"
            :instance="instanceToDelete"
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

import { getAuthContext } from "@/auth_context";
import InstanceCreateModal from "./instance_create_modal.vue";
import InstanceDeleteModal from "./instance_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const instances = ref<{ uuid: string; instanceName: string }[]>([]);
const torii_base_address = ref<string>("");
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const instanceToDelete = ref<{ uuid: string; instanceName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

const emit = defineEmits<{
    (e: "change-view", view: string, instance_uuid: string): void;
}>();

function switchToTasks(instance_uuid: string) {
    const view: string = "WorkloadTask";
    const id: string = instance_uuid;
    emit("change-view", { view, id });
}

async function fetchInstances() {
    try {
        const authContext = getAuthContext();
        torii_base_address.value = authContext.torii_base_address;

        const hanami_api = axios.create({
            baseURL: authContext.hanami_address,
        });

        const response = await hanami_api.get("/v1alpha/instance", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        instances.value = response.data.instances;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load instances");
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
// Add instance modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}

async function acceptAddModal() {
    await fetchInstances();
    cancelAddModal();
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(instance: { uuid: string; instanceName: string }) {
    instanceToDelete.value = instance;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    instanceToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}

async function acceptDeleteModal() {
    await fetchInstances();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchInstances);

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