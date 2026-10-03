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
        <div class="card-label">Members of project {{ props.id }}</div>
        <div class="card-content">
            <!-- Back button -->
            <button class="back-button" @click="switchToProjects">←</button>
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="members.length > 0">
                <thead>
                    <tr>
                        <th>User ID</th>
                        <th>Role</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="member in members" :key="member.user_id">
                        <td>{{ member.user_id }}</td>
                        <td>{{ member.project_role }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(member.user_id)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === member.user_id"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openInfoModal(member)">
                                        Info
                                    </button>
                                    <button @click="openRoleModal(member)">
                                        Change role
                                    </button>
                                    <!-- a user can not be removed from its own
                                         default-project -->
                                    <button
                                        v-if="!isOwnDefaultProject(member)"
                                        @click="openUnassignModal(member)"
                                    >
                                        Delete from project
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <NoEntries v-else text="No members found" />
        </div>

        <ProjectMemberAddModal
            v-if="showAddModal"
            :project_id="props.id"
            :members="members"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <UserInfoModal
            v-if="showInfoModal"
            :user="userToInfo"
            :icons="icons"
            @cancel="cancelInfoModal"
        />

        <ProjectMemberRoleModal
            v-if="showRoleModal"
            :project_id="props.id"
            :member="memberToChange"
            :icons="icons"
            @accept="acceptRoleModal"
            @cancel="cancelRoleModal"
        />

        <ProjectMemberUnassignModal
            v-if="showUnassignModal"
            :project_id="props.id"
            :member="memberToUnassign"
            :icons="icons"
            @accept="acceptUnassignModal"
            @cancel="cancelUnassignModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";

import { miko } from "@/api";
import type { ProjectMemberResp } from "@/api";
import ProjectMemberAddModal from "./project_member_add_modal.vue";
import ProjectMemberRoleModal from "./project_member_role_modal.vue";
import ProjectMemberUnassignModal from "./project_member_unassign_modal.vue";
import UserInfoModal from "../user/user_info_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

const props = defineProps<{
    id: string | null;
}>();

const errorPopupMsg = ref<string>("");
const members = ref<ProjectMemberResp[]>([]);
const openDropdown = ref<string | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;
const memberToChange = ref<ProjectMemberResp | null>(null);
const memberToUnassign = ref<ProjectMemberResp | null>(null);
const showAddModal = ref(false);
const showInfoModal = ref(false);
const userToInfo = ref<{ id: string } | null>(null);

const emit = defineEmits<{
    (e: "change-view", payload: { view: string; id: string | null }): void;
}>();
const showRoleModal = ref(false);
const showUnassignModal = ref(false);

async function fetchMembers() {
    if (!props.id) return;

    try {
        members.value = await miko.listUsersInProject(props.id);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load members");
    }
}

/** Checks, if the project of this view is the default-project of the member. */
function isOwnDefaultProject(member: ProjectMemberResp): boolean {
    return props.id === `default-${member.user_id}`;
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(user_id: string) {
    openDropdown.value = openDropdown.value === user_id ? null : user_id;
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
// Switch back to the list of projects
//=============================================================================
function switchToProjects() {
    emit("change-view", { view: "AdminProject", id: null });
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
    await fetchMembers();
    cancelAddModal();
}

//=============================================================================
// Info modal
//=============================================================================
function openInfoModal(member: ProjectMemberResp) {
    userToInfo.value = { id: member.user_id };
    showInfoModal.value = true;
    openDropdown.value = null;
}

function cancelInfoModal() {
    showInfoModal.value = false;
    userToInfo.value = null;
    openDropdown.value = null;
}

//=============================================================================
// Change role modal
//=============================================================================
function openRoleModal(member: ProjectMemberResp) {
    memberToChange.value = member;
    showRoleModal.value = true;
    openDropdown.value = null;
}

function cancelRoleModal() {
    showRoleModal.value = false;
    memberToChange.value = null;
    openDropdown.value = null; // close any open action dropdown
}

async function acceptRoleModal() {
    await fetchMembers();
    cancelRoleModal();
}

//=============================================================================
// Unassign modal
//=============================================================================
function openUnassignModal(member: ProjectMemberResp) {
    memberToUnassign.value = member;
    showUnassignModal.value = true;
    openDropdown.value = null;
}

function cancelUnassignModal() {
    showUnassignModal.value = false;
    memberToUnassign.value = null;
    openDropdown.value = null; // close any open action dropdown
}

async function acceptUnassignModal() {
    await fetchMembers();
    cancelUnassignModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchMembers);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>
