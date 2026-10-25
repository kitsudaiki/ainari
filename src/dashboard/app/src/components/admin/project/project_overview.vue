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
        <div class="card-label">Project</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="projects.length > 0">
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Lock</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="project in projects" :key="project.id">
                        <td>{{ project.id }}</td>
                        <!-- the reason of the lock, the details are shown by "Lock / unlock" -->
                        <td :class="{ locked: project.lock }">
                            {{ project.lock ? project.lock.lock_reason : "-" }}
                        </td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(project.id)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === project.id"
                                    class="table-dropdown-menu"
                                >
                                    <button
                                        @click="switchToMembers(project.id)"
                                    >
                                        Show members
                                    </button>
                                    <button @click="openLockModal(project)">
                                        Lock / unlock
                                    </button>
                                    <template v-if="isekaiDeployed">
                                        <button @click="openBackupModal(project)">
                                            Create backup
                                        </button>
                                        <button @click="openRestoreModal(project)">
                                            Restore backup
                                        </button>
                                    </template>
                                    <!-- the default-project of a user is only deleted
                                         together with the user -->
                                    <button
                                        v-if="!project.id.startsWith('default-')"
                                        @click="openDeleteModal(project)"
                                    >
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <NoEntries v-else text="No projects found" />
        </div>

        <ProjectCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <ProjectLockModal
            v-if="projectToLock"
            :project="projectToLock"
            :icons="icons"
            @accept="acceptLockModal"
            @cancel="projectToLock = null"
        />

        <BackupCreateModal
            v-if="projectToBackup"
            :project-id="projectToBackup.id"
            :icons="icons"
            @accept="acceptBackupModal"
            @cancel="projectToBackup = null"
        />

        <BackupSelectModal
            v-if="projectToRestore && !backupToRestore"
            :project-id="projectToRestore.id"
            :icons="icons"
            @select="(backup) => (backupToRestore = backup)"
            @cancel="projectToRestore = null"
        />

        <BackupRestoreModal
            v-if="projectToRestore && backupToRestore"
            :backup="backupToRestore"
            :target-project-id="projectToRestore.id"
            :icons="icons"
            @accept="acceptRestoreModal"
            @cancel="closeRestoreModals"
        />

        <ProjectDeleteModal
            v-if="showDeleteModal"
            :project="projectToDelete"
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

import { hasIsekai, miko } from "@/api";
import type { BackupJobResp, BackupResp, JobResp, ProjectBasicResp } from "@/api";
import ProjectCreateModal from "./project_create_modal.vue";
import ProjectDeleteModal from "./project_delete_modal.vue";
import ProjectLockModal from "./project_lock_modal.vue";
import BackupCreateModal from "@/components/storage/backup/backup_create_modal.vue";
import BackupSelectModal from "@/components/storage/backup/backup_select_modal.vue";
import BackupRestoreModal from "@/components/storage/backup/backup_restore_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

const errorPopupMsg = ref<string>("");
const projects = ref<ProjectBasicResp[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const projectToDelete = ref<ProjectBasicResp | null>(null);
const projectToLock = ref<ProjectBasicResp | null>(null);
// backups are only available, if isekai is deployed
const isekaiDeployed = hasIsekai();
const projectToBackup = ref<ProjectBasicResp | null>(null);
const projectToRestore = ref<ProjectBasicResp | null>(null);
const backupToRestore = ref<BackupResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

const emit = defineEmits<{
    (e: "change-view", payload: { view: string; id: string }): void;
}>();

async function fetchProjects() {
    try {
        projects.value = await miko.listProjects();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load projects");
    }
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(id: string) {
    openDropdown.value = openDropdown.value === id ? null : id;
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
// Switch to the members of a project
//=============================================================================
function switchToMembers(project_id: string) {
    emit("change-view", { view: "AdminProjectMember", id: project_id });
}

//=============================================================================
// Add project modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}

function cancelAddModal() {
    showAddModal.value = false;
}

async function acceptAddModal() {
    await fetchProjects();
    cancelAddModal();
}

//=============================================================================
// Delete modal
//=============================================================================
function openLockModal(project: ProjectBasicResp) {
    projectToLock.value = project;
    openDropdown.value = null;
}

async function acceptLockModal() {
    projectToLock.value = null;
    await fetchProjects();
}

//=============================================================================
// Backup and restore of a project
//=============================================================================
function openBackupModal(project: ProjectBasicResp) {
    projectToBackup.value = project;
    openDropdown.value = null;
}

// the backup is created by a job in the background, which is followed in the view of the backups
function acceptBackupModal(result: BackupJobResp) {
    projectToBackup.value = null;
    emit("change-view", { view: "StorageBackup", id: result.backup.uuid });
}

function openRestoreModal(project: ProjectBasicResp) {
    projectToRestore.value = project;
    backupToRestore.value = null;
    openDropdown.value = null;
}

function closeRestoreModals() {
    projectToRestore.value = null;
    backupToRestore.value = null;
}

function acceptRestoreModal(job: JobResp) {
    closeRestoreModals();
    emit("change-view", { view: "StorageBackup", id: job.backup_uuid });
}

function openDeleteModal(project: ProjectBasicResp) {
    projectToDelete.value = project;
    showDeleteModal.value = true;
    openDropdown.value = null;
}

function cancelDeleteModal() {
    showDeleteModal.value = false;
    projectToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}

async function acceptDeleteModal() {
    await fetchProjects();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchProjects);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.locked {
    color: var(--color-usage-warning);
}
</style>
