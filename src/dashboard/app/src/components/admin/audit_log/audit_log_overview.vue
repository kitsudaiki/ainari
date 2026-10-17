<!-- 
// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
    <div class="card">
        <div class="card-label">Audit Log</div>
        <div class="card-content">
            <!-- the newest entries first, like they are returned by omikuji -->
            <table class="overview-table" v-if="auditLogs.length > 0">
                <thead>
                    <tr>
                        <th>Created At</th>
                        <th>Component</th>
                        <th>Host</th>
                        <th>User</th>
                        <th>Project</th>
                        <th>Request</th>
                        <th>Endpoint</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="entry in auditLogs" :key="entry.uuid">
                        <td>{{ common.formatDateTime(entry.created_at) }}</td>
                        <td>{{ entry.component_name }}</td>
                        <td>{{ entry.host_name }}</td>
                        <td>{{ entry.user_id || "-" }}</td>
                        <td>{{ entry.project_id || "-" }}</td>
                        <td>{{ entry.request_type }}</td>
                        <td class="message-column" :title="entry.endpoint">
                            {{ entry.endpoint }}
                        </td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(entry.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === entry.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openInfoModal(entry)">
                                        Info
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <NoEntries v-else text="No audit-log-entries found" />
        </div>

        <AuditLogInfoModal
            v-if="showInfoModal"
            :entry="entryToInfo"
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

import { omikuji } from "@/api";
import type { AuditLogResp } from "@/api";
import AuditLogInfoModal from "./audit_log_info_modal.vue";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

const errorPopupMsg = ref<string>("");
const auditLogs = ref<AuditLogResp[]>([]);
const showInfoModal = ref(false);
const openDropdown = ref<string | null>(null);
const entryToInfo = ref<AuditLogResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchAuditLogs() {
    try {
        auditLogs.value = await omikuji.listAuditLogs();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load the audit-log",
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
function openInfoModal(entry: AuditLogResp) {
    entryToInfo.value = entry;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    entryToInfo.value = null;
    openDropdown.value = null; // close any open action dropdown
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchAuditLogs);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
/* the endpoint can be long, so it is cut in the table and fully shown in the info-modal */
.message-column {
    max-width: 30rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* The first column of the overview-tables is made wide for UUIDs, but here it only holds
the timestamp, so it shrinks to its content. The freed space goes to the user and the
project. */
.overview-table th:nth-child(1),
.overview-table td:nth-child(1) {
    width: 1%;
}

.overview-table th:nth-child(4),
.overview-table td:nth-child(4),
.overview-table th:nth-child(5),
.overview-table td:nth-child(5) {
    width: 20%;
}
</style>
