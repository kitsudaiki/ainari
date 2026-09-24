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
        <div class="card-label">Snapshot</div>
        <div class="card-content">
            <table class="overview-table" v-if="snapshots.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr
                        v-for="snapshot in snapshots"
                        :key="snapshot.uuid"
                    >
                        <td>{{ snapshot.uuid }}</td>
                        <td>{{ snapshot.name }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(snapshot.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === snapshot.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button
                                        @click="openDeleteModal(snapshot)"
                                    >
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No snapshots found</p>
        </div>

        <SnapshotDeleteModal
            v-if="showDeleteModal"
            :snapshot="snapshotToDelete"
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

import { ryokan } from "@/api";
import type { SnapshotBasicResp } from "@/api";

import SnapshotDeleteModal from "./snapshot_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const snapshots = ref<SnapshotBasicResp[]>([]);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const snapshotToDelete = ref<SnapshotBasicResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchSnapshots() {
    try {
        snapshots.value = await ryokan.listSnapshots();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load snapshots",
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
// Delete modal
//=============================================================================
function openDeleteModal(snapshot: SnapshotBasicResp) {
    snapshotToDelete.value = snapshot;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    snapshotToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchSnapshots();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchSnapshots);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>
