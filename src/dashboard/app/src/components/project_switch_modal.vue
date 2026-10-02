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
        <div class="modal project-switch-modal">
            <div class="modal-topbar">
                <span>Switch project</span>
            </div>
            <div class="modal-content">
                <table class="overview-table" v-if="projects.length > 0">
                    <thead>
                        <tr>
                            <th>Project-ID</th>
                            <th>Role</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="project in projects"
                            :key="project.project_id"
                            :class="{
                                'selected-row':
                                    selectedProjectId === project.project_id,
                            }"
                            @click="selectedProjectId = project.project_id"
                        >
                            <td>
                                {{ project.project_id }}
                                <span
                                    v-if="project.project_id === currentProjectId"
                                    class="current-marker"
                                >
                                    (current)
                                </span>
                            </td>
                            <td>{{ project.project_role }}</td>
                        </tr>
                    </tbody>
                </table>
                <p v-else-if="loaded">No projects found</p>

                <p v-if="selectionError" class="error-msg">
                    A project must be selected
                </p>
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
import { ref, onMounted } from "vue";

import { miko } from "@/api";
import type { ProjectInvitedResp } from "@/api";
import {
    createAuthContext,
    getAuthContext,
    getProjectIdFromJwt,
} from "@/auth_context";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept", token: string): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const selectionError = ref(false);
const loaded = ref(false);
const projects = ref<ProjectInvitedResp[]>([]);
const currentProjectId = getProjectIdFromJwt(getAuthContext().token);
// the current project is preselected, so accepting without a change keeps it
const selectedProjectId = ref<string | null>(currentProjectId);

async function fetchProjects() {
    try {
        projects.value = await miko.listInvitedProjects();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load projects");
    } finally {
        loaded.value = true;
    }
}

/**
 * Requests a new token for the selected project and replaces the current token with it, so all
 * following requests are done within the selected project.
 */
async function handleAccept() {
    selectionError.value = selectedProjectId.value === null;
    if (selectedProjectId.value === null) return;

    try {
        const resp = await miko.renewToken({
            project_id: selectedProjectId.value,
        });
        await createAuthContext(resp.access_token);
        emit("accept", resp.access_token);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to switch project");
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchProjects);
</script>

<style scoped>
.project-switch-modal {
    width: 36rem;
}

.overview-table tbody tr {
    cursor: pointer;
}

/* the selected project is filled with the highlight-color */
.overview-table tbody tr.selected-row td {
    background: var(--color-highlight);
    color: var(--color-on-highlight);
}

/* the project-ids are no UUIDs, so the wide mono-space column is not required */
.overview-table td:nth-child(1) {
    width: auto;
}

.current-marker {
    font-family: inherit;
    opacity: 0.7;
    margin-left: 0.5rem;
}
</style>
