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
        <div class="card-label">Tasks</div>
        <div class="card-content">
            <!-- Checkpoint actions, which both create a new task -->
            <div class="task-actions">
                <button @click="showSaveModal = true">Save checkpoint</button>
                <button @click="showRestoreModal = true">
                    Restore checkpoint
                </button>
            </div>

            <table class="overview-table" v-if="tasks.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Type</th>
                        <th>State</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="task in tasks" :key="task.uuid">
                        <td>{{ task.uuid }}</td>
                        <td>{{ task.name }}</td>
                        <td>{{ task.task_type }}</td>
                        <td>
                            <span
                                class="state-badge"
                                :class="stateClass(task.state)"
                            >
                                {{ task.state }}
                            </span>
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
                                    <button @click="openInfoModal(task)">
                                        Info
                                    </button>
                                    <button
                                        v-if="isAbortable(task.state)"
                                        @click="openAbortModal(task)"
                                    >
                                        Abort
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No tasks found</p>
        </div>

        <TaskInfoModal
            v-if="showInfoModal"
            :virtual_machine_uuid="props.id"
            :torii_port="torii_port"
            :task="taskToShow"
            :icons="icons"
            @cancel="cancelInfoModal"
        />

        <TaskAbortModal
            v-if="showAbortModal"
            :virtual_machine_uuid="props.id"
            :torii_port="torii_port"
            :task="taskToAbort"
            :icons="icons"
            @accept="acceptAbortModal"
            @cancel="cancelAbortModal"
        />

        <CheckpointSaveModal
            v-if="showSaveModal"
            :virtual_machine_uuid="props.id"
            :torii_port="torii_port"
            :icons="icons"
            @accept="acceptCheckpointModal"
            @cancel="showSaveModal = false"
        />

        <CheckpointRestoreModal
            v-if="showRestoreModal"
            :virtual_machine_uuid="props.id"
            :torii_port="torii_port"
            :icons="icons"
            @accept="acceptCheckpointModal"
            @cancel="showRestoreModal = false"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";

import { hanami, sakura } from "@/api";
import type { TaskBasicResp, TaskState } from "@/api";
import TaskInfoModal from "./task_info_modal.vue";
import TaskAbortModal from "./task_abort_modal.vue";
import CheckpointSaveModal from "./checkpoint_save_modal.vue";
import CheckpointRestoreModal from "./checkpoint_restore_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const props = defineProps<{
    id: string | null;
}>();

const errorPopupMsg = ref<string>("");
const tasks = ref<TaskBasicResp[]>([]);
const openDropdown = ref<string | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;
const taskToShow = ref<TaskBasicResp | null>(null);
const taskToAbort = ref<TaskBasicResp | null>(null);
const showInfoModal = ref(false);
const showAbortModal = ref(false);
const showSaveModal = ref(false);
const showRestoreModal = ref(false);

// the torii-port of the virtual-machine, which is needed to reach its sakura
const torii_port = ref<number>(0);

// tasks change their state in the background, so the list is polled
let refreshInterval: number | undefined;

/** States, in which a task can still be aborted. */
function isAbortable(state: TaskState): boolean {
    return state === "Created" || state === "Queued" || state === "Active";
}

/** Maps the state of a task to the css-class, which colors its badge. */
function stateClass(state: TaskState): string {
    switch (state) {
        case "Finished":
            return "state-finished";
        case "Active":
            return "state-active";
        case "Error":
        case "Aborted":
            return "state-failed";
        default:
            return "state-pending";
    }
}

async function fetchTasks() {
    if (!props.id) return;

    try {
        // the sakura is only reachable through the torii, so the port of the
        // virtual-machine has to be resolved first
        if (torii_port.value === 0) {
            const virtualMachine = await hanami.getVirtualMachine(props.id);
            torii_port.value = virtualMachine.torii_port;
        }

        tasks.value = await sakura.listTasks(torii_port.value, props.id);
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
// Info modal
//=============================================================================
function openInfoModal(task: TaskBasicResp) {
    taskToShow.value = task;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    taskToShow.value = null;
    openDropdown.value = null;
}

//=============================================================================
// Abort modal
//=============================================================================
function openAbortModal(task: TaskBasicResp) {
    taskToAbort.value = task;
    showAbortModal.value = true;
    openDropdown.value = null;
}
function cancelAbortModal() {
    showAbortModal.value = false;
    taskToAbort.value = null;
    openDropdown.value = null; // close any open action dropdown
}

async function acceptAbortModal() {
    await fetchTasks();
    cancelAbortModal();
}

//=============================================================================
// Checkpoint modals
//=============================================================================
async function acceptCheckpointModal() {
    showSaveModal.value = false;
    showRestoreModal.value = false;
    await fetchTasks();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(async () => {
    await fetchTasks();
    refreshInterval = window.setInterval(fetchTasks, 2000);
});

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    if (refreshInterval) clearInterval(refreshInterval);
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.task-actions {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
}

.overview-table td:nth-child(2) {
    width: 15rem;
}
.overview-table td:nth-child(3) {
    width: 10rem;
}

.state-badge {
    padding: 0.1rem 0.5rem;
    font-size: 0.85rem;
    color: black;
}

.state-finished {
    background-color: #52c41a;
}
.state-active {
    background-color: var(--color-highlight);
    color: var(--color-on-highlight);
}
.state-failed {
    background-color: #ff4d4f;
}
.state-pending {
    background-color: #d9d9d9;
}
</style>
