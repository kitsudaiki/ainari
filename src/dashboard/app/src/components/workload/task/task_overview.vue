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
        <div class="card-label">Task</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openAddModal(props.id)">
                +
            </button>

            <table class="overview-table" v-if="tasks.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Type</th>
                        <th>Progress</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="task in tasks" :key="task.uuid">
                        <td>{{ task.uuid }}</td>
                        <td>{{ task.name }}</td>
                        <td>{{ task.task_type }}</td>
                        <td>
                            <ProgressBar
                                :task_uuid="task.uuid"
                                :cluster_uuid="props.id"
                            />
                        </td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(task.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === task.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openDeleteModal(task)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No tasks found</p>
        </div>

        <TaskCreateModal
            v-if="showAddModal"
            :cluster_uuid="props.id"
            :torii_port="torii_port"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import "primevue/resources/themes/saga-blue/theme.css";
import axios from "axios";

import context from "../../../auth_context";
import TaskCreateModal from "./task_create_modal.vue";
import ProgressBar from "./progress_bar.vue";
import { handleAxiosError } from "@/handleAxiosError";

const props = defineProps<{
    id: string | null;
}>();

const errorPopupMsg = ref<string>("");
const tasks = ref<{ uuid: string; taskName: string }[]>([]);
const showAddModal = ref(false);
const openDropdown = ref<string | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;
var torii_port = 0;

// const logArrayElements = (element, index /*, array */) => {
//   console.log(`a[${index}] = ${element.total_number_of_epochs}`);
// };

async function fetchTasks() {
    try {
        const authContext = context.getAuthContext();
        const hanami_api = axios.create({
            baseURL: authContext.hanami_address,
        });

        // get torii-port of the cluster
        const cluster_response = await hanami_api.get(
            `/v1alpha/cluster/${props.id}`,
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );
        torii_port = cluster_response.data.torii_port;

        const sakura_api = axios.create({
            baseURL: `${authContext.torii_base_address}:${torii_port}`,
        });

        const task_response = await sakura_api.get(
            `/v1alpha/cluster/${props.id}/task`,
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );
        tasks.value = task_response.data.tasks;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load tasks");
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
// Add task modal
//=============================================================================
function openAddModal(cluster_uuid: string) {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}

async function acceptAddModal() {
    await fetchTasks();
    cancelAddModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchTasks);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.overview-table td:nth-child(2) {
    width: 15rem;
}
.overview-table td:nth-child(3) {
    width: 10rem;
}
</style>
