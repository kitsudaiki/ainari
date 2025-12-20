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
    <div class="modal-overlay" @click.self="cancel">
        <div class="modal project-create-modal">
            <div class="modal-topbar">
                <span>Create project</span>
            </div>
            <div class="modal-content">
                <input
                    v-model="form.projectId"
                    type="text"
                    placeholder="Project-ID"
                    required
                />
                <input
                    v-model="form.projectName"
                    type="text"
                    placeholder="Project-Name"
                    required
                />
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
</template>

<script lang="ts" setup>
import { reactive } from "vue";
import axios from "axios";

import context from "../../../auth_context";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    projectId: "",
    projectName: "",
});

async function handleAccept() {
    try {
        const authContext = context.getAuthContext();
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
    } catch (err) {
        console.error("Failed to create project", err);
    }

    console.log("Submitting form:", form);
    emit("accept");
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.project-create-modal {
    height: 18rem;
    width: 20rem;
}
</style>
