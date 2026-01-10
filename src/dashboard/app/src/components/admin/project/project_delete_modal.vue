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
        <div class="modal project-delete-modal">
            <div class="modal-topbar">
                <span>Delete project</span>
            </div>
            <div class="modal-content">
                <p>Are you sure you want to delete?</p>
                <strong>Project: {{ project?.id }}</strong>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        @click="handleAccept(project?.id)"
                    >
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
import axios from "axios";

import context from "../../../auth_context";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    project: { id: number; name: string } | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();
const errorPopupMsg = ref<string>("");

async function handleAccept(project_id: string) {
    if (!project_id) return;
    try {
        const authContext = context.getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        await miko_api.delete(`/v1alpha/project/${project_id}/admin`, {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to delete project");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.project-delete-modal {
    width: 30rem;
}
</style>
