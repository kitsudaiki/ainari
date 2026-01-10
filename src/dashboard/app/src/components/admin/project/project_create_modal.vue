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
        <div class="modal project-create-modal">
            <div class="modal-topbar">
                <span>Create project</span>
            </div>
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.projectId"
                        type="text"
                        placeholder="Project-ID"
                        :class="{ invalid_input: projectIdError }"
                    />
                    <p v-if="projectIdError" class="error-msg">
                        Project-ID must be at least 4 characters
                    </p>
                </div>
                <br />
                <div>
                    <input
                        v-model="form.projectName"
                        type="text"
                        placeholder="Project-Name"
                        :class="{ invalid_input: projectNameError }"
                    />
                    <p v-if="projectNameError" class="error-msg">
                        Project-Name must be at least 4 characters
                    </p>
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
import { reactive } from "vue";
import axios from "axios";

import { getAuthContext } from "@/auth_context";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const projectIdError = ref(false);
const projectNameError = ref(false);

const form = reactive({
    projectId: "",
    projectName: "",
});

async function handleAccept() {
    projectIdError.value = form.userId.length < 4;
    projectNameError.value = form.userName.length < 4;

    if (projectIdError.value || projectNameError.value) {
        return;
    }

    try {
        const authContext = getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        await miko_api.post(
            "/v1alpha/project/admin",
            {
                id: form.projectId,
                name: form.projectName,
            },
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to create project");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.project-create-modal {
    width: 30rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
