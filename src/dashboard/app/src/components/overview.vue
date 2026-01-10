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
    <div class="divider">
        <span>RESOURCE-OVERVIEW</span>
    </div>
    <div class="card">
        <div class="card-label">Cluster</div>
        <div class="card-content">
            <table class="overview-table" v-if="clusters.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="cluster in clusters" :key="cluster.uuid">
                        <td>{{ cluster.uuid }}</td>
                        <td>{{ cluster.name }}</td>
                        <td></td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
    <div class="divider">
        <span>RESOURCE-USAGE</span>
    </div>
    <div class="usage_overview">
        <div class="card gauge-chart-card">
            <div class="card-label">Cluster</div>
            <GaugeChart
                :value="quotaMetrics.clusters.used"
                :max="quotaMetrics.clusters.max"
            />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Datasets</div>
            <GaugeChart
                :value="quotaMetrics.datasets.used"
                :max="quotaMetrics.datasets.max"
            />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Checkpoints</div>
            <GaugeChart
                :value="quotaMetrics.checkpoints.used"
                :max="quotaMetrics.checkpoints.max"
            />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Secrets</div>
            <GaugeChart
                :value="quotaMetrics.secrets.used"
                :max="quotaMetrics.secrets.max"
            />
        </div>
    </div>

    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from "vue";
import axios from "axios";

import context from "../auth_context";
import GaugeChart from "./gauge_chart.vue";
import { handleAxiosError } from "../handleAxiosError";

// Cluster management
const clusters = ref<{ uuid: string; clusterName: string }[]>([]);

// Error handling
const errorPopupMsg = ref<string>("");

// Quota tracking
const quotaMetrics = reactive({
    clusters: {
        used: ref(0),
        max: ref(1),
    },
    datasets: {
        used: ref(0),
        max: ref(1),
    },
    checkpoints: {
        used: ref(0),
        max: ref(1),
    },
    secrets: {
        used: ref(0),
        max: ref(1),
    },
});

// API client creation helper
function createApiClient(baseURL: string | null) {
    const authContext = context.getAuthContext();
    return axios.create({
        baseURL,
        headers: { Authorization: `Bearer ${authContext.token}` },
    });
}

/**
 * Fetches the list of clusters of the user from Hanami API
 */
async function fetchClusters() {
    try {
        const hanamiApi = createApiClient(
            context.getAuthContext().hanami_address,
        );
        const response = await hanamiApi.get("/v1alpha/cluster");
        clusters.value = response.data.clusters;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load clusters");
    }
}

/**
 * Fetches quota limits from Miko API
 */
async function fetchQuotas() {
    try {
        const mikoApi = createApiClient(context.getAuthContext().miko_address);
        const response = await mikoApi.get("/v1alpha/quota");

        quotaMetrics.clusters.max = response.data.max_cluster;
        quotaMetrics.datasets.max = response.data.max_dataset;
        quotaMetrics.checkpoints.max = response.data.max_checkpoint;
        quotaMetrics.secrets.max = response.data.max_secret;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load quotas");
    }
}

/**
 * Fetches the number of used clusters from Hanami API
 */
async function fetchUsedCluster() {
    try {
        const hanamiApi = createApiClient(
            context.getAuthContext().hanami_address,
        );
        const response = await hanamiApi.get("/v1alpha/cluster/count");
        quotaMetrics.clusters.used = response.data.number_of_items;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load number of clusters",
        );
    }
}

/**
 * Fetches the number of used datasets and checkpoints from Ryokan API
 */
async function fetchUsedDatasetsAndCheckpoints() {
    try {
        const ryokanApi = createApiClient(
            context.getAuthContext().ryokan_address,
        );

        // Fetch dataset number
        const respDataset = await ryokanApi.get("/v1alpha/dataset/count");
        quotaMetrics.datasets.used = respDataset.data.number_of_items;

        // Fetch checkpoint number
        const respCheckpoint = await ryokanApi.get("/v1alpha/checkpoint/count");
        quotaMetrics.checkpoints.used = respCheckpoint.data.number_of_items;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load number of datasets and checkpoints",
        );
    }
}

/**
 * Fetches the number of used secrets from Omamori API
 */
async function fetchUsedSecrets() {
    try {
        const omamoriApi = createApiClient(
            context.getAuthContext().omamori_address,
        );
        const response = await omamoriApi.get("/v1alpha/secret/count");
        quotaMetrics.secrets.used = response.data.number_of_items;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load number of secrets",
        );
    }
}

// Initialize all data fetching on component mount
onMounted(() => {
    fetchClusters();
    fetchQuotas();
    fetchUsedCluster();
    fetchUsedDatasetsAndCheckpoints();
    fetchUsedSecrets();
});
</script>

<style scoped>
.usage_overview {
    display: flex;
    flex-wrap: wrap;
}
</style>
