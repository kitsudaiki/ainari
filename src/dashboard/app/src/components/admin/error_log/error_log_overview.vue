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
        <div class="card-label">Error Log</div>
        <div class="card-content">
            <!-- the newest entries first, like they are returned by omikuji -->
            <table class="overview-table" v-if="errorLogs.length > 0">
                <thead>
                    <tr>
                        <th>Created At</th>
                        <th>Component</th>
                        <th>Host</th>
                        <th>User</th>
                        <th>Project</th>
                        <th>Message</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="entry in errorLogs" :key="entry.uuid">
                        <td>{{ common.formatDateTime(entry.created_at) }}</td>
                        <td>{{ entry.component_name }}</td>
                        <td>{{ entry.host_name }}</td>
                        <td>{{ entry.user_id || "-" }}</td>
                        <td>{{ entry.project_id || "-" }}</td>
                        <td class="message-column" :title="entry.message">
                            {{ entry.message }}
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

            <NoEntries v-else text="No error-log-entries found" />
        </div>

        <ErrorLogInfoModal
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
import type { ErrorLogResp } from "@/api";
import ErrorLogInfoModal from "./error_log_info_modal.vue";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

const errorPopupMsg = ref<string>("");
const errorLogs = ref<ErrorLogResp[]>([]);
const showInfoModal = ref(false);
const openDropdown = ref<string | null>(null);
const entryToInfo = ref<ErrorLogResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchErrorLogs() {
    try {
        errorLogs.value = await omikuji.listErrorLogs();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load the error-log",
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
function openInfoModal(entry: ErrorLogResp) {
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
onMounted(fetchErrorLogs);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
/* the message can be long, so it is cut in the table and fully shown in the info-modal */
.message-column {
    max-width: 30rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
</style>
