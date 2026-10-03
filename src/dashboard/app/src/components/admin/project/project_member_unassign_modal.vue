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
        <div class="modal project-member-unassign-modal">
            <div class="modal-topbar">
                <span>Delete user from project</span>
            </div>
            <div class="modal-content">
                <p>Are you sure you want to delete the user from the project?</p>
                <strong>User: {{ member?.user_id }}</strong>
                <br />
                <strong>Project: {{ project_id }}</strong>
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
import type { ProjectMemberResp } from "@/api";
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

async function handleAccept() {
    if (!props.project_id || !props.member) return;
    try {
        await miko.removeUserFromProject(props.project_id, {
            user_id: props.member.user_id,
        });

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to delete user from project",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.project-member-unassign-modal {
    width: 30rem;
}
</style>
