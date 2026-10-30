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

            <!-- no header: every row is a multi-line slot, whose fields carry their own captions -->
            <table class="overview-table host-table" v-if="hosts.length > 0">
                <tbody>
                    <tr v-for="host in hosts" :key="host.uuid">
                        <td class="host-cell">
                            <div class="host-layout">
                                <div class="host-identity">
                                    <div class="host-name-line">
                                        <span class="host-name">{{
                                            host.name
                                        }}</span>
                                        <span
                                            v-if="
                                                isSakuraHost(host) &&
                                                host.is_host_isolated
                                            "
                                            class="host-badge"
                                        >
                                            Isolated
                                        </span>
                                        <span
                                            v-if="
                                                isOnsenHost(host) &&
                                                host.is_backup_target
                                            "
                                            class="host-badge"
                                        >
                                            Backup target
                                        </span>
                                    </div>
                                    <div class="host-address">
                                        {{ host.host_address }}
                                    </div>
                                    <div class="host-uuid">{{ host.uuid }}</div>
                                </div>

                                <div
                                    v-if="isSakuraHost(host)"
                                    class="host-resources"
                                >
                                    <div class="host-resource">
                                        <span class="host-caption">Cores</span>
                                        <UsageBar
                                            :used="host.used_number_of_cores"
                                            :total="host.number_of_cores"
                                        />
                                    </div>
                                    <div class="host-resource">
                                        <span class="host-caption">Memory</span>
                                        <UsageBar
                                            :used="
                                                mibToGib(
                                                    host.amount_of_used_memory,
                                                )
                                            "
                                            :total="mibToGib(host.memory_size)"
                                            unit="GiB"
                                        />
                                    </div>
                                    <div class="host-resource">
                                        <span class="host-caption">Disk</span>
                                        <UsageBar
                                            :used="
                                                host.amount_of_used_disk_space
                                            "
                                            :total="host.disk_space"
                                            unit="GiB"
                                        />
                                    </div>
                                </div>
                            </div>
                        </td>
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
                                    <button
                                        v-if="isSakuraHost(host)"
                                        @click="openIsolationModal(host)"
                                    >
                                        Set host isolation
                                    </button>
                                    <button
                                        v-if="isOnsenHost(host)"
                                        @click="openBackupTargetModal(host)"
                                    >
                                        Set backup target
                                    </button>
                                    <button @click="openDeleteModal(host)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <NoEntries v-else :text="`No ${selectedTab}-hosts found`" />
        </div>

        <HostDeleteModal
            v-if="showDeleteModal"
            :host="hostToDelete"
            :host_kind="selectedTab"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />

        <OnsenBackupTargetModal
            v-if="hostToMark"
            :host="hostToMark"
            :icons="icons"
            @accept="acceptBackupTargetModal"
            @cancel="hostToMark = null"
        />

        <HostIsolationModal
            v-if="showIsolationModal"
            :host="hostToIsolate"
            :icons="icons"
            @accept="acceptIsolationModal"
            @cancel="cancelIsolationModal"
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
import type {
    HostBasicResp,
    OnsenHostBasicResp,
    SakuraHostBasicResp,
} from "@/api";
import HostDeleteModal from "./host_delete_modal.vue";
import HostIsolationModal from "./host_isolation_modal.vue";
import OnsenBackupTargetModal from "./onsen_backup_target_modal.vue";
import UsageBar from "@/components/usage_bar.vue";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

/** The sakura-hosts are known by the hanami, the onsen-hosts by the ryokan. */
type HostKind = "sakura" | "onsen";

const errorPopupMsg = ref<string>("");
// only the sakura-hosts provide their resources
const hosts = ref<(OnsenHostBasicResp | SakuraHostBasicResp)[]>([]);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const hostToDelete = ref<HostBasicResp | null>(null);
const showIsolationModal = ref(false);
const hostToIsolate = ref<SakuraHostBasicResp | null>(null);
const hostToMark = ref<OnsenHostBasicResp | null>(null);
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

// only the onsen-hosts can be backup-targets
function isOnsenHost(
    host: HostBasicResp | OnsenHostBasicResp | SakuraHostBasicResp,
): host is OnsenHostBasicResp {
    return "is_backup_target" in host;
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
// Isolation modal
//=============================================================================
function openIsolationModal(host: SakuraHostBasicResp) {
    hostToIsolate.value = host;
    showIsolationModal.value = true;
    openDropdown.value = null;
}
function cancelIsolationModal() {
    showIsolationModal.value = false;
    hostToIsolate.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptIsolationModal() {
    await fetchHosts();
    cancelIsolationModal();
}

//=============================================================================
// Backup-target modal
//=============================================================================
function openBackupTargetModal(host: OnsenHostBasicResp) {
    hostToMark.value = host;
    openDropdown.value = null;
}
async function acceptBackupTargetModal() {
    await fetchHosts();
    hostToMark.value = null;
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
/* more distance to the tab-slider, because the rows are higher than in other tables */
.host-table {
    margin-top: 1.25rem;
}

/* the host-cell is the first cell, but holds more than the UUID, so the mono-font and the
fixed width of the shared overview-table are reset here */
.overview-table td.host-cell {
    width: auto;
    white-space: normal;
    font-family: var(--title-font-family);
    font-size: 1rem;
    font-weight: 500;
    letter-spacing: 0.02em;
    padding-top: 0.8rem;
    padding-bottom: 0.8rem;
}

/* identity on the left, resources on the right. On small screens the resources wrap below
the identity. */
.host-layout {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem 2.5rem;
}

.host-identity {
    flex: 1 1 18rem;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}

.host-name-line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.6rem;
}

.host-name {
    color: var(--color-text);
    font-size: 1.05rem;
    overflow-wrap: anywhere;
}

.host-address {
    color: var(--color-text-light);
    font-size: 0.85rem;
    overflow-wrap: anywhere;
}

/* the UUID inherits the color of the first cell, so it lights up on hover like in the
other overview-tables */
.host-uuid {
    font-family: var(--mono-font-family);
    font-size: 0.75rem;
    font-weight: 400;
    letter-spacing: 0;
    overflow-wrap: anywhere;
}

/* small outlined flags for the special states of a host */
.host-badge {
    padding: 0.05rem 0.45rem;
    border: 1px solid currentColor;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    white-space: nowrap;
    color: var(--color-highlight);
}

/* the usage-bars get equal columns with enough space to be readable */
.host-resources {
    flex: 3 1 30rem;
    display: grid;
    grid-template-columns: repeat(3, minmax(9rem, 1fr));
    gap: 0.5rem 1.5rem;
}

.host-resource {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
}

.host-caption {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--color-text-light);
}
</style>
