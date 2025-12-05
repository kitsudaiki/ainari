<!-- 
// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
                                <textarea
                                    id="mapping_definition"
                                    v-model="form.inputMapping"
                                    type="text"
                                    required
                                ></textarea>
                            </div>
                            <label> </label>
                            <div>
                                <label>Output-Mapping:</label>
                                <textarea
                                    id="mapping_definition"
                                    v-model="form.outputMapping"
                                    type="text"
                                    required
                                ></textarea>
                            </div>
                        </label>
                    </div>
                    <div v-show="selectedTab === 'Request'">
                        <label>
                            <div>
                                <label>Input-Mapping:</label>
                                <textarea
                                    id="mapping_definition"
                                    v-model="form.inputMapping"
                                    type="text"
                                    required
                                ></textarea>
                            </div>
                            <label> </label>
                            <div>
                                <label>Output-Mapping:</label>
                                <textarea
                                    id="mapping_definition"
                                    v-model="form.outputMapping"
                                    type="text"
                                    required
                                ></textarea>
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
                        @click="handleAccept(cluster_uuid)"
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
import api from "../../../api";

interface Props {
    cluster_uuid: string;
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    inputMapping: "",
    outputMapping: "",
    taskName: "",
});

async function handleAccept(cluster_uuid: string) {
    if (selectedTab.value === "Train") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);

        const inputs = JSON.parse(form.inputMapping);
        const outputs = JSON.parse(form.outputMapping);

        const token = localStorage.getItem("jwtToken");
        await api.sakura_api.post(
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
        // console.log("Upload success!", response.data);
    }

    if (selectedTab.value === "Request") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);

        const inputs = JSON.parse(form.inputMapping);
        const outputs = JSON.parse(form.outputMapping);

        const token = localStorage.getItem("jwtToken");
        await api.sakura_api.post(
            `/v1alpha/cluster/${cluster_uuid}/task/request`,
            {
                name: form.taskName,
                inputs: inputs,
                outputs: outputs,
                time_length: 1,
            },
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        // console.log("Upload success!", response.data);
    }

    if (selectedTab.value === "Checkpoint save") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);

        const token = localStorage.getItem("jwtToken");
        await api.sakura_api.post(
            `/v1alpha/cluster/${cluster_uuid}/task/checkpoint_save`,
            {
                name: form.taskName,
            },
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        // console.log("Upload success!", response.data);
    }

    if (selectedTab.value === "Checkpoint restore") {
        console.log("selected checkpoint-uuid: ", selectedCheckpointUuid.value);

        const token = localStorage.getItem("jwtToken");
        await api.sakura_api.post(
            `/v1alpha/cluster/${cluster_uuid}/task/checkpoint_restore`,
            {
                name: form.taskName,
                checkpoint_uuid: selectedCheckpointUuid.value,
            },
            {
                headers: { Authorization: `Bearer ${token}` },
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
        const token = localStorage.getItem("jwtToken");
        const response = await api.sakura_api.get("/v1alpha/checkpoint", {
            headers: { Authorization: `Bearer ${token}` },
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

onMounted(fetchCheckpoints);
</script>

<style scoped>
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
