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
        <div class="modal project-member-role-modal">
            <div class="modal-topbar">
                <span>Change role of user: {{ member?.user_id }}</span>
            </div>
            <div class="modal-content">
                <strong>Project: {{ project_id }}</strong>
                <br />
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
import { ref } from "vue";

import { miko } from "@/api";
import type { ProjectMemberResp, ProjectRole } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    project_id: string | null;
    member: ProjectMemberResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();
const errorPopupMsg = ref<string>("");

const roles: ProjectRole[] = ["admin", "member", "observer"];

// the current role of the user is preselected
const projectRole = ref<ProjectRole>(props.member?.project_role ?? "member");

async function handleAccept() {
    if (!props.project_id || !props.member) return;
    try {
        await miko.setProjectRole(props.member.user_id, {
            project_id: props.project_id,
            project_role: projectRole.value,
        });

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to change role");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.project-member-role-modal {
    width: 32rem;
}

.field-row {
    display: grid;
    grid-template-columns: 10rem 18rem; /* fixed input width */
    align-items: center;
}
</style>
