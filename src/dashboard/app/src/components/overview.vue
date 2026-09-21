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
        <div class="card-label">Virtual Machines</div>
        <div class="card-content">
            <table class="overview-table" v-if="virtualMachines.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Address</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    <tr
                        v-for="virtualMachine in virtualMachines"
                        :key="virtualMachine.uuid"
                    >
                        <td>{{ virtualMachine.uuid }}</td>
                        <td>{{ virtualMachine.name }}</td>
                        <td>
                            {{ torii_base_address }}:{{
                                virtualMachine.proxy_port
                            }}
                        </td>
                        <td></td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No virtual machines found</p>
        </div>
    </div>
    <div class="divider">
        <span>RESOURCE-USAGE</span>
    </div>
    <div class="usage_overview">
        <div class="card gauge-chart-card">
            <div class="card-label">Virtual Machines</div>
            <GaugeChart
                :value="quotaMetrics.virtualMachines.used"
                :max="quotaMetrics.virtualMachines.max"
            />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Images</div>
            <GaugeChart
                :value="quotaMetrics.images.used"
                :max="quotaMetrics.images.max"
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
        <div class="card gauge-chart-card">
            <div class="card-label">Networks</div>
            <GaugeChart
                :value="quotaMetrics.networks.used"
                :max="quotaMetrics.networks.max"
            />
        </div>
        <div class="card gauge-chart-card">
            <div class="card-label">Floating IPs</div>
            <GaugeChart
                :value="quotaMetrics.floatingIps.used"
                :max="quotaMetrics.floatingIps.max"
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

import { getAuthContext } from "@/auth_context";
import { hanami, miko, omamori, ryokan } from "@/api";
import type { VirtualMachineBasicResp } from "@/api";
import GaugeChart from "@/components/gauge_chart.vue";
import { handleAxiosError } from "@/handleAxiosError";

const virtualMachines = ref<VirtualMachineBasicResp[]>([]);
const torii_base_address = ref<string | null>("");

// Error handling
const errorPopupMsg = ref<string>("");

// Quota tracking
const quotaMetrics = reactive({
    virtualMachines: { used: 0, max: 1 },
    images: { used: 0, max: 1 },
    checkpoints: { used: 0, max: 1 },
    secrets: { used: 0, max: 1 },
    networks: { used: 0, max: 1 },
    floatingIps: { used: 0, max: 1 },
});

/**
 * Fetches the list of virtual machines of the user from the hanami
 */
async function fetchVirtualMachines() {
    try {
        torii_base_address.value = getAuthContext().torii_base_address;
        virtualMachines.value = await hanami.listVirtualMachines();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load virtual machines",
        );
    }
}

/**
 * Fetches the quota-limits of the user from the miko
 */
async function fetchQuotas() {
    try {
        const quota = await miko.getOwnQuota();

        quotaMetrics.virtualMachines.max = quota.max_virtual_machine;
        quotaMetrics.images.max = quota.max_image;
        quotaMetrics.checkpoints.max = quota.max_checkpoint;
        quotaMetrics.secrets.max = quota.max_secret;
        quotaMetrics.networks.max = quota.max_network;
        quotaMetrics.floatingIps.max = quota.max_floating_ip;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load quotas");
    }
}

/**
 * Fetches the used amount of all resources, which are covered by a quota.
 *
 * The hanami has no count-endpoints for networks and floating-ips, so their lists
 * are used instead.
 */
async function fetchUsage() {
    try {
        quotaMetrics.virtualMachines.used =
            await hanami.getVirtualMachineCount();
        quotaMetrics.networks.used = (await hanami.listNetworks()).length;
        quotaMetrics.floatingIps.used = (await hanami.listFloatingIps()).length;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load usage of the hanami-resources",
        );
    }

    try {
        quotaMetrics.images.used = await ryokan.getImageCount();
        quotaMetrics.checkpoints.used = await ryokan.getCheckpointCount();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load number of images and checkpoints",
        );
    }

    try {
        quotaMetrics.secrets.used = await omamori.getSecretCount();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load number of secrets",
        );
    }
}

// Initialize all data fetching on component mount
onMounted(() => {
    fetchVirtualMachines();
    fetchQuotas();
    fetchUsage();
});
</script>

<style scoped>
.usage_overview {
    display: flex;
    flex-wrap: wrap;
}
/* Columns 2 through n-1 share remaining space equally */
th:not(:first-child):not(:last-child),
td:not(:first-child):not(:last-child) {
    width: 30%;
}
</style>
