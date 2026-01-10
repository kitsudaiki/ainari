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
            <!-- Modal topbar -->
            <div class="modal-topbar">
                <span>Create user</span>
            </div>

            <!-- Modal content -->
            <div class="modal-content">
                <div>
                    <div>
                        <input
                            v-model="form.userId"
                            type="text"
                            placeholder="User-ID"
                            :class="{ invalid_input: userIdError }"
                        />
                        <p v-if="userIdError" class="error-msg">
                            User-ID must be at least 4 characters
                        </p>
                    </div>
                    <br />
                    <div>
                        <input
                            v-model="form.userName"
                            type="text"
                            placeholder="User-Name"
                            :class="{ invalid_input: userNameError }"
                        />
                        <p v-if="userNameError" class="error-msg">
                            User-Name must be at least 4 characters
                        </p>
                    </div>
                    <br />
                    <div>
                        <input
                            v-model="form.password"
                            type="password"
                            placeholder="Password"
                            :class="{ invalid_input: passwordError }"
                        />
                        <p v-if="passwordError" class="error-msg">
                            Password must be at least 8 characters
                        </p>
                    </div>
                    <br />
                    <div>
                        <input
                            v-model="form.confirmPassword"
                            type="password"
                            placeholder="Confirm password"
                            :class="{ invalid_input: passwordConfirmError }"
                        />
                        <p v-if="passwordConfirmError" class="error-msg">
                            Password did not match
                        </p>
                    </div>
                    <br />
                    <div>
                        <label class="checkbox-label">
                            <input type="checkbox" v-model="form.isAdmin" />
                            Is Admin
                        </label>
                    </div>
                </div>
            </div>

            <!-- Modal bottombar -->
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
import { ref, reactive, computed } from "vue";
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

const form = reactive({
    userId: "",
    userName: "",
    password: "",
    confirmPassword: "",
    isAdmin: false,
});

const errorPopupMsg = ref<string>("");
const userIdError = ref(false);
const userNameError = ref(false);
const passwordError = ref(false);
const passwordConfirmError = computed(() =>
    form.password !== form.confirmPassword ? true : false,
);

async function handleAccept() {
    userIdError.value = form.userId.length < 4;
    userNameError.value = form.userName.length < 4;
    passwordError.value = form.password.length < 8;

    if (userIdError.value || passwordError.value || passwordError.value) {
        return;
    }
    if (form.password !== form.confirmPassword) {
        passwordConfirmError.value = true;
        return;
    }
    try {
        const authContext = getAuthContext();
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
    width: 30em;
    margin-bottom: 5rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
