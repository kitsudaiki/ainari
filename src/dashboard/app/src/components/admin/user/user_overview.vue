<!-- 
// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
    <div class="overview">
        <div class="card">
            <div class="card-label">User</div>
            <div class="card-content">
                <!-- Add button -->
                <button class="add-button" @click="openAddModal">+</button>

                <table class="overview-table" v-if="users.length > 0">
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Username</th>
                            <th>Is Admin</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="user in users" :key="user.id">
                            <td>{{ user.id }}</td>
                            <td>{{ user.name }}</td>
                            <td>{{ user.is_admin }}</td>
                            <td>
                                <!-- Dropdown menu -->
                                <div
                                    class="table-dropdown"
                                    @click.stop="toggleDropdown(user.id)"
                                >
                                    ⋮
                                    <div
                                        v-if="openDropdown === user.id"
                                        class="table-dropdown-menu"
                                    >
                                        <button @click="openInfoModal(user)">
                                            Info
                                        </button>
                                        <button @click="openDeleteModal(user)">
                                            Delete
                                        </button>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    </tbody>
                </table>

                <p v-else>No users found</p>
            </div>
        </div>

        <UserCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <UserDeleteModal
            v-if="showDeleteModal"
            :user="userToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />

        <UserInfoModal
            v-if="showInfoModal"
            :user="userToInfo"
            :icons="icons"
            @cancel="cancelInfoModal"
        />
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";
import axios from "axios";

import context from "../../../auth_context";

import UserCreateModal from "./user_create_modal.vue";
import UserDeleteModal from "./user_delete_modal.vue";
import UserInfoModal from "./user_info_modal.vue";

const users = ref<{ id: string; userName: string }[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const showInfoModal = ref(false);
const openDropdown = ref<string | null>(null);
const userToDelete = ref<{ id: string; userName: string } | null>(null);
const userToInfo = ref<{ id: string; userName: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchUsers() {
    try {
        const authContext = context.getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        const response = await miko_api.get("/v1alpha/user/admin", {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        users.value = response.data.users;
    } catch (err) {
        console.error("Failed to load users", err);
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
// Add user modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}

async function acceptAddModal() {
    await fetchUsers();
    cancelAddModal();
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(user: { id: string; userName: string }) {
    userToDelete.value = user;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    userToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchUsers();
    cancelDeleteModal();
}

//=============================================================================
// Info modal
//=============================================================================
function openInfoModal(user: { id: string; userName: string }) {
    userToInfo.value = user;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    userToInfo.value = null;
    openDropdown.value = null; // close any open action dropdown
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchUsers);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>
