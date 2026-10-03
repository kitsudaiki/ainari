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
    <div class="modal-overlay" @click.self="cancel">
        <div class="modal project-member-add-modal">
            <div class="modal-topbar">
                <span>Add user to project: {{ project_id }}</span>
            </div>
            <div class="modal-content">
                <div class="field-row">
                    <label for="user_id">User: </label>
                    <select
                        id="user_id"
                        v-model="userId"
                        class="select-dropdown"
                        :class="{ invalid_input: userError }"
                    >
                        <option value="" disabled>Select a user</option>
                        <option
                            v-for="user in availableUsers"
                            :key="user.id"
                            :value="user.id"
                        >
                            {{ user.id }}
                        </option>
                    </select>
                </div>
                <p v-if="userError" class="error-msg">
                    A user must be selected
                </p>
                <br />
                <div class="field-row">
                    <label for="project_role">Role: </label>
                    <select
                        id="project_role"
                        v-model="projectRole"
                        class="select-dropdown"
                    >
                        <option
                            v-for="role in roles"
                            :key="role"
                            :value="role"
                        >
                            {{ role }}
                        </option>
                    </select>
                </div>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button class="icon-button" @click="handleAccept">
                        <img :src="icons.acceptIcon" alt="Accept" />
                    </button>
                    <button class="icon-button" @click="cancel">
                        <img :src="icons.cancelIcon" alt="Cancel" />
                    </button>
                </div>
            </div>
        </div>
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted } from "vue";

import { miko } from "@/api";
import type { ProjectMemberResp, ProjectRole, UserBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    project_id: string | null;
    members: ProjectMemberResp[];
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();
const errorPopupMsg = ref<string>("");
const userError = ref(false);
const userId = ref<string>("");
const users = ref<UserBasicResp[]>([]);

const roles: ProjectRole[] = ["admin", "member", "observer"];
const projectRole = ref<ProjectRole>("observer");

// users, which are already in the project, can not be added a second time
const availableUsers = computed(() =>
    users.value.filter(
        (user) => !props.members.some((member) => member.user_id === user.id),
    ),
);

async function fetchUsers() {
    try {
        users.value = await miko.listUsers();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load users");
    }
}

async function handleAccept() {
    if (!props.project_id) return;
    userError.value = userId.value === "";
    if (userError.value) return;

    try {
        await miko.addUserToProject(props.project_id, {
            user_id: userId.value,
            project_role: projectRole.value,
        });
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to add user to project",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchUsers);
</script>

<style scoped>
.project-member-add-modal {
    width: 32rem;
}

.field-row {
    display: grid;
    grid-template-columns: 10rem 18rem; /* fixed input width */
    align-items: center;
}
</style>
