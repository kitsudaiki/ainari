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
        <div class="card-label">Floating IPs</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="floatingIps.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Floating IP</th>
                        <th>Internal IP</th>
                        <th>Network</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr
                        v-for="floatingIp in floatingIps"
                        :key="floatingIp.uuid"
                    >
                        <td>{{ floatingIp.uuid }}</td>
                        <td>{{ floatingIp.floating_ip }}</td>
                        <td>{{ floatingIp.internal_ip }}</td>
                        <td>{{ networkName(floatingIp.network_uuid) }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(floatingIp.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === floatingIp.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button
                                        @click="openDeleteModal(floatingIp)"
                                    >
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No floating IPs found</p>
        </div>

        <FloatingIpCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <FloatingIpDeleteModal
            v-if="showDeleteModal"
            :floating_ip="floatingIpToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";

import { hanami } from "@/api";
import type { FloatingIpBasicResp, NetworkBasicResp } from "@/api";
import FloatingIpCreateModal from "./floating_ip_create_modal.vue";
import FloatingIpDeleteModal from "./floating_ip_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const floatingIps = ref<FloatingIpBasicResp[]>([]);
const networks = ref<NetworkBasicResp[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const floatingIpToDelete = ref<FloatingIpBasicResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

/**
 * The list-endpoint only provides the uuid of the network, so the networks are
 * loaded as well to be able to show their name.
 */
function networkName(network_uuid: string): string {
    const network = networks.value.find((entry) => entry.uuid === network_uuid);
    return network ? network.name : network_uuid;
}

async function fetchFloatingIps() {
    try {
        floatingIps.value = await hanami.listFloatingIps();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load floating IPs",
        );
    }

    try {
        networks.value = await hanami.listNetworks();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load networks");
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
// Add floating-ip modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}
async function acceptAddModal() {
    await fetchFloatingIps();
    cancelAddModal();
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(floatingIp: FloatingIpBasicResp) {
    floatingIpToDelete.value = floatingIp;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    floatingIpToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchFloatingIps();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchFloatingIps);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.overview-table td:nth-child(2),
.overview-table td:nth-child(3) {
    width: 12rem;
}
</style>
