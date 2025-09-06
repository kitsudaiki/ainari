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
            <div class="card-label">Users</div>
            <div class="card-content">
                <!-- Add button -->
                <button class="add-button" @click="openAddModal">+</button>

                <table v-if="users.length > 0">
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

        <!-- Add User Modal -->
        <div
            v-if="showAddModal"
            class="modal-overlay"
            @click.self="cancelAddModal"
        >
            <div class="modal user-create-modal">
                <!-- Modal topbar -->
                <div class="modal-topbar">
                    <span>Create user</span>
                </div>
                <div class="modal-content">
                    <label>
                        <input
                            v-model="newUser.userid"
                            type="text"
                            placeholder="User-ID"
                            required
                        />
                    </label>
                    <label>
                        <input
                            v-model="newUser.username"
                            type="text"
                            placeholder="User-Name"
                            required
                        />
                    </label>
                    <label>
                        <input
                            v-model="newUser.password"
                            type="password"
                            placeholder="Password"
                            required
                        />
                    </label>
                    <label>
                        <input
                            v-model="newUser.confirmPassword"
                            type="password"
                            placeholder="Confirm password"
                            required
                        />
                    </label>
                    <label class="checkbox-label">
                        <input type="checkbox" v-model="newUser.isAdmin" />
                        Is Admin
                    </label>

                    <p v-if="passwordError" class="error-msg">
                        {{ passwordError }}
                    </p>
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
            <div class="modal user-delete-modal">
                <div class="modal-topbar">
                    <span>Delete user</span>
                </div>
                <div class="modal-content">
                    <p>Are you sure you want to delete?</p>
                    <strong>User: {{ userToDelete?.id }}</strong>
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
import api from "../api";

const users = ref<{ id: number; username: string; email: string }[]>([]);
const showAddModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<number | null>(null);
const newUser = ref({
    userid: "",
    username: "",
    password: "",
    confirmPassword: "",
    isAdmin: false,
});
const passwordError = ref("");
const userToDelete = ref<{ id: number; username: string } | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchUsers() {
    try {
        const token = localStorage.getItem("jwtToken");
        const response = await api.get("/v1alpha/user", {
            headers: { Authorization: `Bearer ${token}` },
        });
        users.value = response.data.users;
    } catch (err) {
        console.error("Failed to load users", err);
    }
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(id: number) {
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
    newUser.value.userid = "";
    newUser.value.username = "";
    newUser.value.password = "";
    newUser.value.confirmPassword = "";
    newUser.value.isAdmin = false;
    passwordError.value = "";
}

async function acceptAddModal() {
    if (newUser.value.password !== newUser.value.confirmPassword) {
        passwordError.value = "Passwords do not match!";
        return;
    }
    try {
        const token = localStorage.getItem("jwtToken");
        await api.post(
            "/v1alpha/user",
            {
                id: newUser.value.userid,
                name: newUser.value.username,
                passphrase: newUser.value.password,
                is_admin: newUser.value.isAdmin,
            },
            {
                headers: { Authorization: `Bearer ${token}` },
            },
        );
        await fetchUsers();
        cancelAddModal();
    } catch (err) {
        passwordError.value = err;
        console.error("Failed to create user", err);
    }
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(user: { id: number; username: string }) {
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
    if (!userToDelete.value) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.delete(`/v1alpha/user/${userToDelete.value.id}`, {
            headers: { Authorization: `Bearer ${token}` },
        });
        await fetchUsers();
    } catch (err) {
        console.error("Failed to delete user", err);
    } finally {
        cancelDeleteModal();
    }
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

<style scoped>
.user-create-modal {
    height: 28rem;
    width: 20rem;
}

.user-delete-modal {
    height: 16rem;
    width: 20rem;
}
</style>
