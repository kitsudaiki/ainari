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
    <div class="name-item">
        <label>{{ itemName }}</label>
        <select
            v-model="selectedDatasetUuid"
            class="select-dropdown"
            @change="onDatasetSelected"
        >
            <option
                v-for="item in datasets"
                :key="item.uuid"
                :value="item.uuid"
            >
                {{ item.name }} [ {{ item.uuid }} ]
            </option>
        </select>
        <select v-model="selectedColumn" class="select-dropdown">
            <option v-for="item in datasetColumns" :key="item" :value="item">
                {{ item }}
            </option>
        </select>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import axios from "axios";

import context from "../../../auth_context";

interface Props {
    itemName: string;
    datasets: string[];
}
const props = defineProps<Props>();

const selectedDatasetUuid = ref<string>("");
const datasets = ref<{ uuid: string; datasetName: string }[]>([]);

const selectedColumn = ref<string>("");
const datasetColumns = ref<{ columnName: string }[]>([]);

function getData() {
    return {
        hexagon: props.itemName,
        dataset_uuid: selectedDatasetUuid.value,
        dataset_column: selectedColumn.value,
    };
}
defineExpose({
    getData,
});

async function fetchDatasetColumns() {
    try {
        const authContext = context.getAuthContext();

        const ryokan_api = axios.create({
            baseURL: authContext.ryokan_address,
        });

        const resp = await ryokan_api.get(
            `/v1alpha/dataset/${selectedDatasetUuid.value}`,
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );

        datasetColumns.value = resp.data.column_names;
        if (datasetColumns.value.length > 0) {
            selectedColumn.value = datasetColumns.value[0]; // default to first item
        }
    } catch (err) {
        console.error("Failed to load dataset-columns", err);
    }
}

async function onDatasetSelected() {
    // console.log("Selected:", selectedDatasetUuid.value);
    await fetchDatasetColumns();
}

async function fetchDataset() {
    datasets.value = props.datasets;
    if (props.datasets.length > 0) {
        selectedDatasetUuid.value = props.datasets[0].uuid; // default to first item
        await fetchDatasetColumns();
    }
}

onMounted(fetchDataset);
</script>

<style scoped>
.name-item {
    padding: 8px;
    border-bottom: 1px solid #ddd;
}
</style>
