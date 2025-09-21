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
            <div class="card-label">Task</div>
            <div class="card-content">
                <!-- Add button -->
                <button class="add-button" @click="openAddModal">+</button>

                <table class="overview-table" v-if="tasks.length > 0">
                    <thead>
                        <tr>
                            <th>UUID</th>
                            <th>Name</th>
                            <th>Type</th>
                            <th>State</th>
                            <th>Progress</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="task in tasks" :key="task.uuid">
                            <td>{{ task.uuid }}</td>
                            <td>{{ task.name }}</td>
                            <td>{{ task.task_type }}</td>
                            <td>{{ task.state }}</td>
                            <td>
                                <ProgressBar
                                    :value="
                                        parseFloat(
                                            (
                                                (task.current_epoch /
                                                    task.total_number_of_epochs) *
                                                100
                                            ).toFixed(1),
                                        )
                                    "
                                ></ProgressBar>
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
        </div>

        <!-- Add Task Modal -->
        <div
            v-if="showAddModal"
            class="modal-overlay"
            @click.self="cancelAddModal"
        >
            <div class="modal task-create-modal">
                <!-- Modal topbar -->
                <div class="modal-topbar">
                    <span>Create task</span>
                </div>
                <div class="modal-content">
                    <input
                        v-model="newTask.taskName"
                        type="text"
                        placeholder="Task-Name"
                        required
                    />
                    <div>
                        <label>Task template:</label>
                        <textarea
                            id="template_input"
                            v-model="newTask.taskTemplate"
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
            <div class="modal task-delete-modal">
                <div class="modal-topbar">
                    <span>Delete task</span>
                </div>
                <div class="modal-content">
                    <p>Are you sure you want to delete?</p>
                    <strong>Task: {{ taskToDelete?.uuid }}</strong>
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
import "primevue/resources/themes/saga-blue/theme.css"; // or any other theme
import ProgressBar from "primevue/progressbar";

const props = defineProps<{
    id: string | null;
}>();

const tasks = ref<{ uuid: string; taskName: string }[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const newTask = ref({
    taskTemplate: "",
    taskName: "",
});
const passwordError = ref("");
const taskToDelete = ref<{ uuid: string; taskName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

// const logArrayElements = (element, index /*, array */) => {
//   console.log(`a[${index}] = ${element.total_number_of_epochs}`);
// };

async function fetchTasks() {
    console.log("cluster-uuid: ", props.id);
    try {
        const token = localStorage.getItem("jwtToken");
        const response = await api.hanami_api.get(
            `/v1alpha/cluster/${props.id}/task`,
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        tasks.value = response.data.tasks;
        // tasks.value.forEach(logArrayElements);
    } catch (err) {
        console.error("Failed to load tasks", err);
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
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
    newTask.value.taskTemplate = "";
    newTask.value.taskName = "";
}

async function acceptAddModal() {
    try {
        const token = localStorage.getItem("jwtToken");
        await api.hanami_api.post(
            "/v1alpha/task",
            {
                name: newTask.value.taskName,
                template: newTask.value.taskTemplate,
            },
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchTasks();
        cancelAddModal();
    } catch (err) {
        passwordError.value = err;
        console.error("Failed to create task", err);
    }
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(task: { uuid: string; taskName: string }) {
    taskToDelete.value = task;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    taskToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    if (!taskToDelete.value) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.hanami_api.delete(
            `/v1alpha/task/${taskToDelete.value.uuid}`,
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchTasks();
    } catch (err) {
        console.error("Failed to delete task", err);
    } finally {
        cancelDeleteModal();
    }
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
.task-create-modal {
    height: 35rem;
    width: 30rem;
}

.task-delete-modal {
    height: 16rem;
    width: 20rem;
}

.overview-table td:nth-child(3) {
    width: 10rem;
}

.overview-table td:nth-child(4) {
    width: 10rem;
}

.overview-table td:nth-child(2) {
    width: 15rem;
}
</style>
