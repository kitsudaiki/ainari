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
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="project in projects" :key="project.id">
                        <td>{{ project.id }}</td>
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
                                    <button @click="openDeleteModal(project)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No projects found</p>
        </div>

        <ProjectCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
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
import axios from "axios";

import { getAuthContext } from "@/auth_context";
import ProjectCreateModal from "./project_create_modal.vue";
import ProjectDeleteModal from "./project_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const projects = ref<{ id: string; projectName: string }[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const projectToDelete = ref<{ id: string; projectName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchProjects() {
    try {
        const authContext = getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        const response = await miko_api.get("/v1alpha/project/admin", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        projects.value = response.data.projects;
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
function openDeleteModal(project: { id: string; projectName: string }) {
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
