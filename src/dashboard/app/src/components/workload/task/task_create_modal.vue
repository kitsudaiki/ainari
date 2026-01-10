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
    <div class="modal-overlay" @click.self="cancel">
        <div class="modal task-create-modal">
            <div class="modal-topbar">
                <span>Create task</span>
            </div>
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.taskName"
                        type="text"
                        placeholder="Task-Name"
                        :class="{ invalid_input: taskNameError }"
                    />
                    <p v-if="taskNameError" class="error-msg">
                        Task-Name must be at least 4 characters
                    </p>
                </div>
                <br />
                <div class="tab">
                    <button
                        class="tablinks"
                        :class="{ active: isSelected('Train') }"
                        @click="selectTab('Train')"
                    >
                        Train
                    </button>
                    <button
                        class="tablinks"
                        :class="{ active: isSelected('Request') }"
                        @click="selectTab('Request')"
                    >
                        Request
                    </button>
                    <button
                        class="tablinks"
                        :class="{ active: isSelected('Checkpoint save') }"
                        @click="selectTab('Checkpoint save')"
                    >
                        Checkpoint save
                    </button>
                    <button
                        class="tablinks"
                        :class="{ active: isSelected('Checkpoint restore') }"
                        @click="selectTab('Checkpoint restore')"
                    >
                        Checkpoint restore
                    </button>
                </div>
                <div class="task-tabcontent">
                    <div v-show="selectedTab === 'Train'">
                        <label>
                            <div>
                                <label>Input-Mapping:</label>
                                <div class="scroll-container">
                                    <TaskIoItem
                                        v-for="itemName in form.cluster_inputs"
                                        :key="itemName"
                                        ref="inputItems"
                                        :itemName="itemName"
                                        :datasets="form.datasets"
                                    />
                                </div>
                            </div>
                            <label> </label>
                            <div>
                                <label>Output-Mapping:</label>
                                <div class="scroll-container">
                                    <TaskIoItem
                                        v-for="itemName in form.cluster_outputs"
                                        :key="itemName"
                                        ref="outputItems"
                                        :itemName="itemName"
                                        :datasets="form.datasets"
                                    />
                                </div>
                            </div>
                        </label>
                    </div>
                    <div v-show="selectedTab === 'Request'">
                        <label>
                            <div>
                                <label>Input-Mapping:</label>
                                <div class="scroll-container">
                                    <TaskIoItem
                                        v-for="itemName in form.cluster_inputs"
                                        :key="itemName"
                                        ref="inputItems"
                                        :itemName="itemName"
                                        :datasets="form.datasets"
                                    />
                                </div>
                            </div>
                            <label> </label>
                            <div>
                                <label>Output-Mapping:</label>
                                <div class="scroll-container">
                                    <TaskResultItem
                                        v-for="itemName in form.cluster_outputs"
                                        :key="itemName"
                                        ref="resultItems"
                                        :itemName="itemName"
                                    />
                                </div>
                            </div>
                        </label>
                    </div>
                    <div v-show="selectedTab === 'Checkpoint save'"></div>
                    <div v-show="selectedTab === 'Checkpoint restore'">
                        <br />
                        <h5>Select checkpoint:</h5>
                        <br />
                        <select
                            v-model="selectedCheckpointUuid"
                            class="select-dropdown"
                        >
                            <option
                                v-for="item in checkpoints"
                                :key="item.uuid"
                                :value="item.uuid"
                            >
                                {{ item.name }} [ {{ item.uuid }} ]
                            </option>
                        </select>
                    </div>
                </div>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        @click="handleAccept(cluster_uuid, torii_port)"
                    >
                        <img :src="icons.acceptIcon" alt="Accept" />
                    </button>
                    <button class="icon-button" @click="cancel">
                        <img :src="icons.cancelIcon" alt="Cancel" />
                    </button>
                </div>
            </div>
        </div>
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted } from "vue";
import axios from "axios";

import TaskIoItem from "./task_io_item.vue";
import TaskResultItem from "./task_result_item.vue";
import context from "../../../auth_context";
import { handleAxiosError } from "@/handleAxiosError";

/**
 * @property cluster_uuid - Unique identifier for the cluster
 * @property torii_port - Port number for the Torii service
 * @property icons - Object containing icon paths for UI elements
 */
interface Props {
    cluster_uuid: string;
    torii_port: number;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();

// Refs for storing dynamic component data
const inputItems = ref<any[]>([]);
const outputItems = ref<any[]>([]);
const resultItems = ref<any[]>([]);
const errorPopupMsg = ref<string>("");
const taskNameError = ref(false);

// Emits events to parent component
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

// Reactive form data
const form = reactive({
    inputMapping: "",
    outputMapping: "",
    taskName: "",
    cluster_inputs: [],
    cluster_outputs: [],
    datasets: [],
});

/**
 * Handles the accept action for creating a new task

 * @param cluster_uuid - Cluster UUID
 * @param torii_port - Torii service port
 */
async function handleAccept(cluster_uuid: string, torii_port: number) {
    // Validate task name length
    taskNameError.value = form.taskName.length < 4;

    if (taskNameError.value) {
        return;
    }

    try {
        // Handle different task types based on selected tab
        if (selectedTab.value === "Train") {
            // Train task creation
            const authContext = context.getAuthContext();
            const inputs = inputItems.value.map((item) => item.getData());
            const outputs = outputItems.value.map((item) => item.getData());
            const sakura_api = axios.create({
                baseURL: `${authContext.torii_base_address}:${torii_port}`,
            });

            const response = await sakura_api.post(
                `/v1alpha/cluster/${cluster_uuid}/task/train`,
                {
                    name: form.taskName,
                    number_of_epochs: 1,
                    inputs: inputs,
                    outputs: outputs,
                    time_length: 1,
                },
                {
                    headers: {
                        Authorization: `Bearer ${authContext.token}`,
                    },
                },
            );
        }

        if (selectedTab.value === "Request") {
            // Request task creation
            const authContext = context.getAuthContext();
            const inputs = inputItems.value.map((item) => item.getData());
            const results = resultItems.value.map((item) => item.getData());
            const sakura_api = axios.create({
                baseURL: `${authContext.torii_base_address}:${torii_port}`,
            });

            await sakura_api.post(
                `/v1alpha/cluster/${cluster_uuid}/task/request`,
                {
                    name: form.taskName,
                    inputs: inputs,
                    outputs: results,
                    time_length: 1,
                },
                {
                    headers: { Authorization: `Bearer ${authContext.token}` },
                },
            );
        }

        if (selectedTab.value === "Checkpoint save") {
            // Checkpoint save task creation
            const authContext = context.getAuthContext();
            const sakura_api = axios.create({
                baseURL: `${authContext.torii_base_address}:${torii_port}`,
            });

            await sakura_api.post(
                `/v1alpha/cluster/${cluster_uuid}/task/checkpoint_save`,
                {
                    name: form.taskName,
                },
                {
                    headers: { Authorization: `Bearer ${authContext.token}` },
                },
            );
        }

        if (selectedTab.value === "Checkpoint restore") {
            // Checkpoint restore task creation
            const authContext = context.getAuthContext();
            const sakura_api = axios.create({
                baseURL: `${authContext.torii_base_address}:${torii_port}`,
            });

            await sakura_api.post(
                `/v1alpha/cluster/${cluster_uuid}/task/checkpoint_restore`,
                {
                    name: form.taskName,
                    checkpoint_uuid: selectedCheckpointUuid.value,
                },
                {
                    headers: { Authorization: `Bearer ${authContext.token}` },
                },
            );
        }

        // Notify parent component of successful task creation
        emit("accept");
    } catch (err) {
        // Handle and display any errors that occur during task creation
        errorPopupMsg.value = handleAxiosError(err, "Failed to create task");
    }
}

/**
 * Handles the cancel action
 * Notifies parent component that task creation was cancelled
 */
function cancel() {
    emit("cancel");
}

/**
 * Current selected tab in the UI
 * Determines which type of task is being created
 */
const selectedTab = ref<
    "Train" | "Request" | "Checkpoint save" | "Checkpoint restore"
>("Train");

/**
 * Selects a tab in the UI

 * @param tab - The tab to select
 */
function selectTab(
    tab: "Train" | "Request" | "Checkpoint save" | "Checkpoint restore",
) {
    selectedTab.value = tab;
}

/**
 * Checks if a tab is currently selected

 * @param tab - The tab to check

 * @returns true if the tab is selected, false otherwise
 */
function isSelected(
    tab: "Train" | "Request" | "Checkpoint save" | "Checkpoint restore",
) {
    return selectedTab.value === tab;
}

/**
 * List of available checkpoints
 * Used for checkpoint restore operations
 */
const checkpoints = ref<{ uuid: string; checkpointName: string }[]>([]);

/**
 * Currently selected checkpoint UUID
 * Used when creating a checkpoint restore task
 */
const selectedCheckpointUuid = ref<string>("");

/**
 * Fetches available checkpoints from the Ryokan service
 */
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
        if (response.data.checkpoints.length > 0) {
            selectedCheckpointUuid.value = checkpoints.value[0].uuid; // default to first item
        }
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load checkpoints",
        );
    }
}

/**
 * Fetches cluster input and output information from the Hanami service
 */
async function fetchClusterIo() {
    try {
        const authContext = context.getAuthContext();

        const hanami_api = axios.create({
            baseURL: authContext.hanami_address,
        });

        const resp = await hanami_api.get(
            `/v1alpha/cluster/${props.cluster_uuid}`,
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );

        form.cluster_inputs = resp.data.inputs;
        form.cluster_outputs = resp.data.outputs;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load cluster input- and output-names",
        );
    }
}

/**
 * Fetches available datasets from the Ryokan service
 */
async function fetchDatasets() {
    try {
        const authContext = context.getAuthContext();

        const ryokan_api = axios.create({
            baseURL: authContext.ryokan_address,
        });

        const resp = await ryokan_api.get(`/v1alpha/dataset`, {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });

        form.datasets = resp.data.datasets;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load datasets");
    }
}

// Initialize component by fetching required data when mounted
onMounted(fetchClusterIo);
onMounted(fetchDatasets);
onMounted(fetchCheckpoints);
</script>

<style scoped>
.scroll-container {
    max-height: 200px;
    overflow-y: auto;
    border: 1px solid #ccc;
}

.task-create-modal {
    height: 45rem;
    width: 40rem;
}

#mapping_definition {
    height: 10rem;
}

.task-tabcontent {
    margin-top: 0.5rem;
    height: 24rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
