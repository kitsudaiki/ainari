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
        <div class="modal user-create-modal">
            <div class="modal-topbar">
                <span>Create user</span>
            </div>
            <div class="modal-content">
                <input
                    v-model="form.userId"
                    type="text"
                    placeholder="User-ID"
                    required
                />
                <input
                    v-model="form.userName"
                    type="text"
                    placeholder="User-Name"
                    required
                />
                <input
                    v-model="form.password"
                    type="password"
                    placeholder="Password"
                    required
                />
                <input
                    v-model="form.confirmPassword"
                    type="password"
                    placeholder="Confirm password"
                    required
                />
                <label class="checkbox-label">
                    <input type="checkbox" v-model="form.isAdmin" />
                    Is Admin
                </label>

                <p v-if="passwordError" class="error-msg">
                    {{ passwordError }}
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
import { reactive, computed } from "vue";
import axios from "axios";

import context from "../../../auth_context";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    userId: "",
    userName: "",
    password: "",
    confirmPassword: "",
    isAdmin: false,
});

const errorPopupMsg = ref<string>("");

const passwordError = computed(() =>
    form.password !== form.confirmPassword ? "Passwords do not match" : "",
);
import { handleAxiosError } from "@/handleAxiosError";

async function handleAccept() {
    if (form.password !== form.confirmPassword) {
        passwordError.value = "Passwords do not match!";
        return;
    }
    try {
        const authContext = context.getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        await miko_api.post(
            "/v1alpha/user/admin",
            {
                id: form.userId,
                name: form.userName,
                passphrase: form.password,
                is_admin: form.isAdmin.toString(),
            },
            {
                headers: { Authorization: `Bearer ${authContext.token}` },
            },
        );

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to create user");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.user-create-modal {
    height: 28rem;
    width: 20rem;
}
</style>
