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
                <input
                    v-model="form.taskName"
                    type="text"
                    placeholder="Task-Name"
                    required
                />
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
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted } from "vue";
import axios from "axios";

import TaskIoItem from "./task_io_item.vue";
import TaskResultItem from "./task_result_item.vue";
import context from "../../../auth_context";

interface Props {
    cluster_uuid: string;
    torii_port: number;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();

const inputItems = ref<any[]>([]);
const outputItems = ref<any[]>([]);
const resultItems = ref<any[]>([]);

const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    inputMapping: "",
    outputMapping: "",
    taskName: "",
    cluster_inputs: [],
    cluster_outputs: [],
    datasets: [],
});

async function handleAccept(cluster_uuid: string, torii_port: number) {
    if (selectedTab.value === "Train") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);

        const authContext = context.getAuthContext();
        const inputs = inputItems.value.map((item) => item.getData());
        const outputs = outputItems.value.map((item) => item.getData());

        // console.log("+inputs", inputs);
        // console.log("outputs: ", outputs);

        const sakura_api = axios.create({
            baseURL: `${authContext.torii_base_address}:${torii_port}`,
        });

        try {
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
                    headers: { Authorization: `Bearer ${authContext.token}` },
                },
            );
            console.log("Upload success!", response.data);
        } catch (err) {
            console.error("Failed to load dataset-columns", err);
        }
    }

    if (selectedTab.value === "Request") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);
        const authContext = context.getAuthContext();

        const inputs = inputItems.value.map((item) => item.getData());
        const results = resultItems.value.map((item) => item.getData());

        // console.log("+inputs", inputs);
        // console.log("results: ", results);

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
        // console.log("Upload success!", response.data);
    }

    if (selectedTab.value === "Checkpoint save") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);

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
        // console.log("Upload success!", response.data);
    }

    if (selectedTab.value === "Checkpoint restore") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);
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
        // console.log("Upload success!", response.data);
    }

    console.log("Submitting form:", form);
    emit("accept");
}

function cancel() {
    emit("cancel");
}

//=============================================================================
// Tabs
//=============================================================================
const selectedTab = ref<
    "Train" | "Request" | "Checkpoint save" | "Checkpoint restore"
>("Train");

function selectTab(
    tab: "Train" | "Request" | "Checkpoint save" | "Checkpoint restore",
) {
    selectedTab.value = tab;
}

function isSelected(
    tab: "Train" | "Request" | "Checkpoint save" | "Checkpoint restore",
) {
    return selectedTab.value === tab;
}

const checkpoints = ref<{ uuid: string; checkpointName: string }[]>([]);
const selectedCheckpointUuid = ref<string>("");

async function fetchCheckpoints() {
    try {
        const authContext = context.getAuthContext();

        const ryokan_api = axios.create({
            baseURL: authContext.ryokan_address,
        });

        const response = await ryokan_api.get("/v1alpha/checkpoint", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        console.log(response);
        checkpoints.value = response.data.checkpoints;
        console.log(checkpoints.value);
        if (response.data.checkpoints.length > 0) {
            selectedCheckpointUuid.value = checkpoints.value[0].uuid; // default to first item
        }
    } catch (err) {
        console.error("Failed to load checkpoints", err);
    }
}

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
        console.log("Loaded inputs: ", form.cluster_inputs);
        console.log("Loaded outputs: ", form.cluster_outputs);
    } catch (err) {
        console.error("Failed to load cluster input- and output-names", err);
    }
}

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
        console.log("Loaded datasets: ", form.datasets);
    } catch (err) {
        console.error("Failed to load datasets", err);
    }
}

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
</style>
