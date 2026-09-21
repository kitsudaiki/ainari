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
        <div class="card-label">Proxy Entries</div>
        <div class="card-content">
            <table class="overview-table" v-if="proxies.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Virtual Machine</th>
                        <th>Address</th>
                        <th>Target</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="proxy in proxies" :key="proxy.uuid">
                        <td>{{ proxy.uuid }}</td>
                        <td>
                            {{
                                virtualMachineName(proxy.virtual_machine_uuid)
                            }}
                        </td>
                        <td>{{ torii_base_address }}:{{ proxy.port }}</td>
                        <td>{{ proxy.target_address }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(proxy.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === proxy.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openInfoModal(proxy)">
                                        Info
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No proxy entries found</p>
        </div>

        <ProxyInfoModal
            v-if="showInfoModal"
            :proxy="proxyToShow"
            :icons="icons"
            @cancel="cancelInfoModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";

import { getAuthContext } from "@/auth_context";
import { hanami, torii } from "@/api";
import type { ProxyBasicResp, VirtualMachineBasicResp } from "@/api";
import ProxyInfoModal from "./proxy_info_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const proxies = ref<ProxyBasicResp[]>([]);
const virtualMachines = ref<VirtualMachineBasicResp[]>([]);
const torii_base_address = ref<string | null>("");
const showInfoModal = ref(false);
const openDropdown = ref<string | null>(null);
const proxyToShow = ref<ProxyBasicResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

/**
 * A proxy-entry only provides the uuid of its virtual-machine, so the
 * virtual-machines are loaded as well to be able to show their name.
 */
function virtualMachineName(virtual_machine_uuid: string): string {
    const virtualMachine = virtualMachines.value.find(
        (entry) => entry.uuid === virtual_machine_uuid,
    );
    return virtualMachine ? virtualMachine.name : virtual_machine_uuid;
}

async function fetchProxies() {
    torii_base_address.value = getAuthContext().torii_base_address;

    try {
        proxies.value = await torii.listProxies();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load proxy entries",
        );
    }

    try {
        virtualMachines.value = await hanami.listVirtualMachines();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load virtual machines",
        );
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
function openInfoModal(proxy: ProxyBasicResp) {
    proxyToShow.value = proxy;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    proxyToShow.value = null;
    openDropdown.value = null;
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchProxies);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.overview-table td:nth-child(3) {
    width: 14rem;
}
</style>
