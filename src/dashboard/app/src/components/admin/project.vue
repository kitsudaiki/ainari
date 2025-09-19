<!-- 
// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
    <div class="overview">
        <div class="card">
            <div class="card-label">Project</div>
            <div class="card-content">
                <!-- Add button -->
                <button class="add-button" @click="openAddModal">+</button>

                <table v-if="projects.length > 0">
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
                                        <button
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

                <p v-else>No projects found</p>
            </div>
        </div>

        <!-- Add Project Modal -->
        <div
            v-if="showAddModal"
            class="modal-overlay"
            @click.self="cancelAddModal"
        >
            <div class="modal project-create-modal">
                <!-- Modal topbar -->
                <div class="modal-topbar">
                    <span>Create project</span>
                </div>
                <div class="modal-content">
                    <input
                        v-model="newProject.projectId"
                        type="text"
                        placeholder="Project-ID"
                        required
                    />
                    <input
                        v-model="newProject.projectName"
                        type="text"
                        placeholder="Project-Name"
                        required
                    />
                </div>

                <div class="modal-bottombar">
                    <div class="modal-actions">
                        <button class="icon-button" @click="acceptAddModal">
                            <img :src="icons.acceptIcon" alt="Accept" />
                        </button>
                        <button class="icon-button" @click="cancelAddModal">
                            <img :src="icons.cancelIcon" alt="Cancel" />
                        </button>
                    </div>
                </div>
            </div>
        </div>

        <!-- Delete Confirmation Modal -->
        <div
            v-if="showDeleteModal"
            class="modal-overlay"
            @click.self="cancelDeleteModal"
        >
            <div class="modal project-delete-modal">
                <div class="modal-topbar">
                    <span>Delete project</span>
                </div>
                <div class="modal-content">
                    <p>Are you sure you want to delete?</p>
                    <strong>Project: {{ projectToDelete?.id }}</strong>
                </div>

                <div class="modal-bottombar">
                    <div class="modal-actions">
                        <button class="icon-button" @click="acceptDeleteModal">
                            <img :src="icons.acceptIcon" alt="Accept" />
                        </button>
                        <button class="icon-button" @click="cancelDeleteModal">
                            <img :src="icons.cancelIcon" alt="Cancel" />
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import api from "../../api";

const projects = ref<{ id: string; projectName: string; email: string }[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const newProject = ref({
    projectId: "",
    projectName: "",
});
const passwordError = ref("");
const projectToDelete = ref<{ id: string; projectName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchProjects() {
    try {
        const token = localStorage.getItem("jwtToken");
        const response = await api.torii_api.get("/v1alpha/project", {
            headers: { Authorization: `Bearer ${token}` },
        });
        projects.value = response.data.projects;
    } catch (err) {
        console.error("Failed to load projects", err);
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
    newProject.value.projectId = "";
    newProject.value.projectName = "";
    newProject.value.password = "";
    newProject.value.confirmPassword = "";
    newProject.value.isAdmin = false;
    passwordError.value = "";
}

async function acceptAddModal() {
    if (newProject.value.password !== newProject.value.confirmPassword) {
        passwordError.value = "Passwords do not match!";
        return;
    }
    try {
        const token = localStorage.getItem("jwtToken");
        await api.torii_api.post(
            "/v1alpha/project",
            {
                id: newProject.value.projectId,
                name: newProject.value.projectName,
                passphrase: newProject.value.password,
                is_admin: newProject.value.isAdmin,
            },
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchProjects();
        cancelAddModal();
    } catch (err) {
        passwordError.value = err;
        console.error("Failed to create project", err);
    }
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
    if (!projectToDelete.value) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.torii_api.delete(
            `/v1alpha/project/${projectToDelete.value.id}`,
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchProjects();
    } catch (err) {
        console.error("Failed to delete project", err);
    } finally {
        cancelDeleteModal();
    }
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
.project-create-modal {
    height: 18rem;
    width: 20rem;
}

.project-delete-modal {
    height: 16rem;
    width: 20rem;
}
</style>
