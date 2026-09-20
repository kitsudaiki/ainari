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
        <div class="modal instance-create-modal">
            <div class="modal-topbar">
                <span>Create instance</span>
            </div>
            <div class="modal-content">
                <div>
                    <input
                        v-instance="form.instanceName"
                        type="text"
                        placeholder="Instance-Name"
                        :class="{ invalid_input: instanceNameError }"
                    />
                    <p v-if="instanceNameError" class="error-msg">
                        Instance-Name must be at least 4 characters
                    </p>
                </div>
                <br />
                <div>
                    <label>Instance template:</label>
                    <textarea
                        id="template_input"
                        v-instance="form.instanceTemplate"
                        type="text"
                        :class="{ invalid_input: instanceTemplateError }"
                    ></textarea>
                    <p v-if="instanceTemplateError" class="error-msg">
                        Instance-Template is not allowed to left empty
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
import { ref, reactive } from "vue";
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
const instanceNameError = ref(false);
const instanceTemplateError = ref(false);

const form = reactive({
    instanceTemplate: "",
    instanceName: "",
});

async function handleAccept() {
    instanceNameError.value = form.instanceName.length < 4;
    instanceTemplateError.value = form.instanceTemplate.length === 0;

    if (instanceNameError.value || instanceTemplateError.value) {
        return;
    }

    try {
        const authContext = getAuthContext();
        const hanami_api = axios.create({
            baseURL: authContext.hanami_address,
        });

        await hanami_api.post(
            "/v1alpha/instance",
            {
                name: form.instanceName,
                template: form.instanceTemplate,
            },
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to create instance");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.instance-create-modal {
    min-width: 30rem;
}

#template_input {
    height: 18rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
