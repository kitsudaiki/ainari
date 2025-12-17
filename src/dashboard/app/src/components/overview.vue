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
    <div class="divider">
        <span>RESOURCE-USAGE</span>
    </div>
    <div class="usage_overview">
        <div class="card gauge-chart-card">
            <div class="card-label">Cluster</div>
            <GaugeChart :value="usedCluster" :max="maxCluster" />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Datasets</div>
            <GaugeChart :value="usedDataset" :max="maxDataset" />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Checkpoints</div>
            <GaugeChart :value="usedCheckpoint" :max="maxCheckpoint" />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Secrets</div>
            <GaugeChart :value="usedSecret" :max="maxSecret" />
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import axios from "axios";

import context from "../auth_context";

import GaugeChart from "./gauge_chart.vue";

const usedCluster = ref(0);
const maxCluster = ref(1);
const usedDataset = ref(0);
const maxDataset = ref(1);
const usedCheckpoint = ref(0);
const maxCheckpoint = ref(1);
const usedSecret = ref(0);
const maxSecret = ref(1);

async function fetchQuotas() {
    try {
        const authContext = context.getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        const response = await miko_api.get("/v1alpha/quota", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });

        maxCluster.value = response.data.max_cluster;
        maxDataset.value = response.data.max_dataset;
        maxCheckpoint.value = response.data.max_checkpoint;
        maxSecret.value = response.data.max_secret;
    } catch (err) {
        console.error("Failed to load quotas", err);
    }
}

async function fetchUsedCluster() {
    try {
        const authContext = context.getAuthContext();
        const hanami_api = axios.create({
            baseURL: authContext.hanami_address,
        });

        const response = await hanami_api.get("/v1alpha/cluster/count", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        console.log("response: ", response);
        usedCluster.value = response.data.number_of_items;
    } catch (err) {
        console.error("Failed to load number of clusters", err);
    }
}

async function fetchUsedDatasetsAndCheckpoints() {
    try {
        const authContext = context.getAuthContext();
        const ryokan_api = axios.create({
            baseURL: authContext.ryokan_address,
        });

        const respDataset = await ryokan_api.get("/v1alpha/dataset/count", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        console.log("response: ", respDataset);
        usedDataset.value = respDataset.data.number_of_items;

        const respCheckpoint = await ryokan_api.get(
            "/v1alpha/checkpoint/count",
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );
        console.log("response: ", respCheckpoint);
        usedCheckpoint.value = respCheckpoint.data.number_of_items;
    } catch (err) {
        console.error("Failed to load number of datasets and checkpoints", err);
    }
}

async function fetchUsedSecrets() {
    try {
        const authContext = context.getAuthContext();
        const omamori_api = axios.create({
            baseURL: authContext.omamori_address,
        });

        const response = await omamori_api.get("/v1alpha/secret/count", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        console.log("response: ", response);
        usedSecret.value = response.data.number_of_items;
    } catch (err) {
        console.error("Failed to load number of secrets", err);
    }
}

onMounted(fetchQuotas);
onMounted(fetchUsedCluster);
onMounted(fetchUsedDatasetsAndCheckpoints);
onMounted(fetchUsedSecrets);
</script>

<style scoped>
.usage_overview {
    display: flex;
    flex-wrap: wrap; /* 👈 allows wrapping */
}
</style>
