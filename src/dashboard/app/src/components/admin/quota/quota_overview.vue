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
        <div class="card-label">Quota</div>
        <div class="card-content">
            <table class="overview-table" v-if="quotas.length > 0">
                <thead>
                    <tr>
                        <th>User-ID</th>
                        <th>Max Models</th>
                        <th>Max Datasets</th>
                        <th>Max Checkpoints</th>
                        <th>Max Secrets</th>
                        <th>Max Task-Queue</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="quota in quotas" :key="quota.user_id">
                        <td>{{ quota.user_id }}</td>
                        <td>{{ quota.max_model }}</td>
                        <td>{{ quota.max_dataset }}</td>
                        <td>{{ quota.max_checkpoint }}</td>
                        <td>{{ quota.max_secret }}</td>
                        <td>{{ quota.max_taskqueue }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(quota.user_id)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === quota.user_id"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openUpdateModal(quota)">
                                        Change Quota
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No quotas found</p>
        </div>

        <QuotaUpdateModal
            v-if="showUpdateModal"
            :quota="quotaToUpdate"
            :icons="icons"
            @accept="acceptUpdateModal"
            @cancel="cancelUpdateModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import axios from "axios";

import { getAuthContext } from "@/auth_context";
import QuotaUpdateModal from "./quota_update_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const quotas = ref<{ user_id: string }[]>([]);
const showUpdateModal = ref(false);
const openDropdown = ref<string | null>(null);
const quotaToUpdate = ref<{
    user_id: string;
    max_model: number;
    max_dataset: number;
    max_checkpoint: number;
    max_secret: number;
    max_taskqueue: number;
} | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchQuotas() {
    try {
        const authContext = getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        const response = await miko_api.get("/v1alpha/quota/admin", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        quotas.value = response.data.quotas;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load quotas");
    }
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(user_id: string) {
    openDropdown.value = openDropdown.value === user_id ? null : user_id;
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
// Update modal
//=============================================================================
function openUpdateModal(quota: {
    user_id: string;
    max_model: number;
    max_dataset: number;
    max_checkpoint: number;
    max_secret: number;
    max_taskqueue: number;
}) {
    quotaToUpdate.value = quota;
    showUpdateModal.value = true;
    openDropdown.value = null;
}

function cancelUpdateModal() {
    showUpdateModal.value = false;
    quotaToUpdate.value = null;
    openDropdown.value = null; // close any open action dropdown
}

async function acceptUpdateModal() {
    await fetchQuotas();
    cancelUpdateModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchQuotas);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
/* Columns 2 through n-1 share remaining space equally */
th:not(:first-child):not(:last-child),
td:not(:first-child):not(:last-child) {
    width: 15%;
}
</style>
