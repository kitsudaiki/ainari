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
        <div class="card-label">Hosts</div>
        <div class="card-content">
            <!-- Hosts register themselves, so they are only grouped by their kind here -->
            <div class="tab">
                <button
                    class="tablinks"
                    :class="{ active: selectedTab === 'sakura' }"
                    @click="selectTab('sakura')"
                >
                    SAKURA
                </button>
                <button
                    class="tablinks"
                    :class="{ active: selectedTab === 'onsen' }"
                    @click="selectTab('onsen')"
                >
                    ONSEN
                </button>
            </div>

            <table class="overview-table" v-if="hosts.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Address</th>
                        <template v-if="selectedTab === 'sakura'">
                            <th class="resource-column">Cores</th>
                            <th class="resource-column">Memory</th>
                            <th class="resource-column">Disk</th>
                        </template>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="host in hosts" :key="host.uuid">
                        <td>{{ host.uuid }}</td>
                        <td>{{ host.name }}</td>
                        <td>{{ host.host_address }}</td>
                        <template v-if="isSakuraHost(host)">
                            <td class="resource-column">
                                <UsageBar
                                    :used="host.used_number_of_cores"
                                    :total="host.number_of_cores"
                                />
                            </td>
                            <td class="resource-column">
                                <UsageBar
                                    :used="mibToGib(host.amount_of_used_memory)"
                                    :total="mibToGib(host.memory_size)"
                                    unit="GiB"
                                />
                            </td>
                            <td class="resource-column">
                                <UsageBar
                                    :used="host.amount_of_used_disk_space"
                                    :total="host.disk_space"
                                    unit="GiB"
                                />
                            </td>
                        </template>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(host.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === host.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openDeleteModal(host)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No {{ selectedTab }}-hosts found</p>
        </div>

        <HostDeleteModal
            v-if="showDeleteModal"
            :host="hostToDelete"
            :host_kind="selectedTab"
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

import { hanami, ryokan } from "@/api";
import type { HostBasicResp, SakuraHostBasicResp } from "@/api";
import HostDeleteModal from "./host_delete_modal.vue";
import UsageBar from "@/components/usage_bar.vue";
import { handleAxiosError } from "@/handleAxiosError";

/** The sakura-hosts are known by the hanami, the onsen-hosts by the ryokan. */
type HostKind = "sakura" | "onsen";

const errorPopupMsg = ref<string>("");
// only the sakura-hosts provide their resources
const hosts = ref<(HostBasicResp | SakuraHostBasicResp)[]>([]);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const hostToDelete = ref<HostBasicResp | null>(null);
const selectedTab = ref<HostKind>("sakura");
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchHosts() {
    try {
        hosts.value =
            selectedTab.value === "sakura"
                ? await hanami.listSakuraHosts()
                : await ryokan.listOnsenHosts();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            `Failed to load ${selectedTab.value}-hosts`,
        );
    }
}

//=============================================================================
// Resources
//=============================================================================
function isSakuraHost(
    host: HostBasicResp | SakuraHostBasicResp,
): host is SakuraHostBasicResp {
    return "number_of_cores" in host;
}

/** The memory is provided in MiB, but GiB with one decimal is easier to read. */
function mibToGib(mib: number): number {
    return Math.round((mib / 1024) * 10) / 10;
}

//=============================================================================
// Tabs
//=============================================================================
async function selectTab(tab: HostKind) {
    selectedTab.value = tab;
    hosts.value = [];
    await fetchHosts();
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
// Delete modal
//=============================================================================
function openDeleteModal(host: HostBasicResp) {
    hostToDelete.value = host;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    hostToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchHosts();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchHosts);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
/* Columns 2 through n-1 share remaining space equally, except the resource-columns */
th:not(:first-child):not(:last-child):not(.resource-column),
td:not(:first-child):not(:last-child):not(.resource-column) {
    width: 30%;
}

.resource-column {
    width: 10rem;
}
</style>
