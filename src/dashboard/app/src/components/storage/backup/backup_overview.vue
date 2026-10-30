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
        <div class="card-label">Backups</div>
        <div class="card-content">
            <template v-if="isekaiDeployed">
                <!-- Add button, which creates a backup of the whole project -->
                <button
                    class="add-button"
                    title="Create a backup of the project"
                    @click="showCreateModal = true"
                >
                    +
                </button>

                <div class="tab">
                    <button
                        class="tablinks"
                        :class="{ active: selectedTab === 'backups' }"
                        @click="selectedTab = 'backups'"
                    >
                        BACKUPS
                    </button>
                    <button
                        class="tablinks"
                        :class="{ active: selectedTab === 'jobs' }"
                        @click="selectedTab = 'jobs'"
                    >
                        JOBS
                    </button>
                </div>

                <!-- backups -->
                <template v-if="selectedTab === 'backups'">
                    <table class="overview-table" v-if="backups.length > 0">
                        <thead>
                            <tr>
                                <th>UUID</th>
                                <th>Name</th>
                                <th>Kind</th>
                                <th>State</th>
                                <th>Size</th>
                                <th>Created At</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="backup in backups" :key="backup.uuid">
                                <td>{{ backup.uuid }}</td>
                                <td>{{ backup.name }}</td>
                                <td>{{ backupKindLabel(backup) }}</td>
                                <td>{{ backup.state }}</td>
                                <td>
                                    {{
                                        backup.total_size !== null
                                            ? common.formatFileSize(backup.total_size)
                                            : "-"
                                    }}
                                </td>
                                <td>{{ common.formatDateTime(backup.created_at) }}</td>
                                <td>
                                    <div
                                        class="table-dropdown"
                                        @click.stop="toggleDropdown(backup.uuid)"
                                    >
                                        ⋮
                                        <div
                                            v-if="openDropdown === backup.uuid"
                                            class="table-dropdown-menu"
                                        >
                                            <button @click="openInfoModal(backup)">
                                                Info
                                            </button>
                                            <button
                                                v-if="backup.state === 'READY'"
                                                @click="openRestoreModal(backup)"
                                            >
                                                Restore
                                            </button>
                                            <button @click="showJobsOf(backup.uuid)">
                                                Show jobs
                                            </button>
                                            <button @click="openDeleteModal(backup)">
                                                Delete
                                            </button>
                                        </div>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                    <NoEntries v-else text="No backups found" />
                </template>

                <!-- jobs -->
                <template v-else>
                    <p v-if="jobFilter" class="job-filter">
                        Jobs of backup {{ backupName(jobFilter) }}
                        <button
                            class="error-close-btn job-filter-clear"
                            title="Show all jobs"
                            @click="jobFilter = null"
                        >
                            ✕
                        </button>
                    </p>
                    <table class="overview-table" v-if="filteredJobs.length > 0">
                        <thead>
                            <tr>
                                <th>UUID</th>
                                <th>Type</th>
                                <th>Backup</th>
                                <th>Project</th>
                                <th>State</th>
                                <th>Created At</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-for="job in filteredJobs" :key="job.uuid">
                                <td>{{ job.uuid }}</td>
                                <td>{{ job.job_type }}</td>
                                <td>{{ backupName(job.backup_uuid) }}</td>
                                <td>{{ job.project_id }}</td>
                                <td>{{ job.state }}</td>
                                <td>{{ common.formatDateTime(job.created_at) }}</td>
                                <td>
                                    <div
                                        class="table-dropdown"
                                        @click.stop="toggleDropdown(job.uuid)"
                                    >
                                        ⋮
                                        <div
                                            v-if="openDropdown === job.uuid"
                                            class="table-dropdown-menu"
                                        >
                                            <button @click="openJobInfoModal(job)">
                                                Info
                                            </button>
                                        </div>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                    <NoEntries v-else text="No jobs found" />
                </template>
            </template>

            <NoEntries
                v-else
                text="Isekai is not deployed, so there are no backups"
            />
        </div>

        <BackupCreateModal
            v-if="showCreateModal"
            :icons="icons"
            @accept="acceptCreateModal"
            @cancel="showCreateModal = false"
        />

        <BackupInfoModal
            v-if="backupToShow"
            :backup="backupToShow"
            :icons="icons"
            @cancel="backupToShow = null"
        />

        <BackupRestoreModal
            v-if="backupToRestore"
            :backup="backupToRestore"
            :icons="icons"
            @accept="acceptRestoreModal"
            @cancel="backupToRestore = null"
        />

        <BackupDeleteModal
            v-if="backupToDelete"
            :backup="backupToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="backupToDelete = null"
        />

        <BackupJobInfoModal
            v-if="jobToShow"
            :job="jobToShow"
            :icons="icons"
            @cancel="jobToShow = null"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, inject } from "vue";

import { hasIsekai, isekai } from "@/api";
import type { BackupJobResp, BackupResp, JobResp } from "@/api";
import common from "@/common";
import BackupCreateModal from "./backup_create_modal.vue";
import BackupInfoModal from "./backup_info_modal.vue";
import BackupRestoreModal from "./backup_restore_modal.vue";
import BackupDeleteModal from "./backup_delete_modal.vue";
import BackupJobInfoModal from "./backup_job_info_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

/** A backup, whose jobs are shown, like after a new backup of a virtual machine. */
const props = defineProps<{
    id?: string | null;
}>();

type BackupTab = "backups" | "jobs";

const errorPopupMsg = ref<string>("");
const isekaiDeployed = hasIsekai();
const backups = ref<BackupResp[]>([]);
const jobs = ref<JobResp[]>([]);
const selectedTab = ref<BackupTab>(props.id ? "jobs" : "backups");
const jobFilter = ref<string | null>(props.id ?? null);
const openDropdown = ref<string | null>(null);
const showCreateModal = ref(false);
const backupToShow = ref<BackupResp | null>(null);
const backupToRestore = ref<BackupResp | null>(null);
const backupToDelete = ref<BackupResp | null>(null);
const jobToShow = ref<JobResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

const filteredJobs = computed(() =>
    jobFilter.value
        ? jobs.value.filter((job) => job.backup_uuid === jobFilter.value)
        : jobs.value,
);

async function fetchAll() {
    if (!isekaiDeployed) return;
    try {
        [backups.value, jobs.value] = await Promise.all([
            isekai.listBackups(),
            isekai.listJobs(),
        ]);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load backups");
    }
}

function backupKindLabel(backup: BackupResp): string {
    return backup.kind === "PROJECT" ? "Project" : "Virtual Machine";
}

/** The name of a backup, or its UUID, if the backup is not listed, like a deleted one. */
function backupName(uuid: string): string {
    return backups.value.find((backup) => backup.uuid === uuid)?.name ?? uuid;
}

function showJobsOf(uuid: string) {
    jobFilter.value = uuid;
    selectedTab.value = "jobs";
    openDropdown.value = null;
}

//=============================================================================
// Refresh while backups or jobs are running
//=============================================================================
const REFRESH_INTERVAL_MS = 5000;
let refreshTimer: ReturnType<typeof setInterval> | null = null;

// the jobs run in the background, so the lists are reloaded, until all of them are done
function refreshWhilePending() {
    const pending =
        backups.value.some((backup) => ["CREATING", "DELETING"].includes(backup.state)) ||
        jobs.value.some((job) => ["QUEUED", "ACTIVE"].includes(job.state));
    if (pending && refreshTimer === null) {
        refreshTimer = setInterval(fetchAll, REFRESH_INTERVAL_MS);
    } else if (!pending && refreshTimer !== null) {
        clearInterval(refreshTimer);
        refreshTimer = null;
    }
}
watch([backups, jobs], refreshWhilePending);

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
// Modals
//=============================================================================
async function acceptCreateModal(result: BackupJobResp) {
    showCreateModal.value = false;
    await fetchAll();
    showJobsOf(result.backup.uuid);
}

function openInfoModal(backup: BackupResp) {
    backupToShow.value = backup;
    openDropdown.value = null;
}

function openRestoreModal(backup: BackupResp) {
    backupToRestore.value = backup;
    openDropdown.value = null;
}

async function acceptRestoreModal(job: JobResp) {
    backupToRestore.value = null;
    await fetchAll();
    showJobsOf(job.backup_uuid);
}

function openDeleteModal(backup: BackupResp) {
    backupToDelete.value = backup;
    openDropdown.value = null;
}

async function acceptDeleteModal() {
    backupToDelete.value = null;
    await fetchAll();
}

function openJobInfoModal(job: JobResp) {
    jobToShow.value = job;
    openDropdown.value = null;
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchAll);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
    if (refreshTimer !== null) {
        clearInterval(refreshTimer);
    }
});
</script>

<style scoped>
/* the tab-bar ends at the floating add-button, so the margin keeps them apart */
.add-button {
    margin-left: 0.75rem;
}

.job-filter {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.75rem;
}

/* the close-button of the error-popup is positioned absolutely, here it flows with the text */
.job-filter-clear {
    position: static;
}
</style>
