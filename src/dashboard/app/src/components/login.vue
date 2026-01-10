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
    <div class="login-overlay">
        <div class="modal login-modal">
            <!-- Modal topbar -->
            <div class="modal-topbar">
                <span>Login</span>
            </div>

            <!-- Modal content -->
            <div class="modal-content">
                <div>
                    <input
                        v-model="user_id"
                        type="text"
                        id="login_id_field"
                        placeholder="User-ID"
                        :class="{ invalid_input: userIdError }"
                    />
                    <p v-if="userIdError" class="error-msg">
                        User-ID must be at least 4 characters
                    </p>
                </div>

                <br /><br />

                <div>
                    <input
                        v-model="password"
                        type="password"
                        id="login_pw_field"
                        placeholder="Password"
                        :class="{ invalid_input: passwordError }"
                    />
                    <p v-if="passwordError" class="error-msg">
                        Password must be at least 8 characters
                    </p>
                </div>
            </div>

            <!-- Modal bottombar -->
            <div class="modal-bottombar">
                <button @click="login">Login</button>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import axios from "axios";

import { createAuthContext, getIsAdminFromJwt, getExpireTimesamp } from "@/auth_context";
import { getConfig } from "@/config";

// Define custom events that this component can emit
const emit = defineEmits<{
    (
        e: "login-success",
        token: string,
        userId: string,
        isAdmin: boolean,
        expireTimestamp: number,
    ): void;
}>();

// Reactive references to store user input and validation states
const user_id = ref("");
const password = ref("");
const error = ref("");
const userIdError = ref(false);
const passwordError = ref(false);

/**
 * Validates the user input before attempting login
 * @returns boolean - true if validation passes, false otherwise
 */
function validateInput(): boolean {
    userIdError.value = user_id.value.length < 4;
    passwordError.value = password.value.length < 8;
    return !(userIdError.value || passwordError.value);
}

/**
 * Attempts to authenticate the user with the provided credentials
 * and establish an authenticated session.
 */
async function login() {
    try {
        // Reset error state
        error.value = "";

        // Validate input before proceeding
        if (!validateInput()) {
            return;
        }

        // Prepare authentication request parameters
        const params = new URLSearchParams();
        params.append("grant_type", "client_credentials");
        params.append("token_format", "jwt");
        params.append("client_id", user_id.value);
        params.append("client_secret", password.value);

        // Configure axios instance with base URL of Miko from the config
        const { apiUrl } = getConfig();
        const miko_api = axios.create({
            baseURL: apiUrl,
        });

        // Send the authentication request to the API endpoint
        // The endpoint expects form-encoded data with specific headers
        const login_resp = await miko_api.post("/v1alpha/token", params, {
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
            },
        });

        // Extract token from response
        const token = login_resp.data.access_token;

        // Store the authentication token in the global context
        // This makes the token available to other components in the application
        await createAuthContext(token);
        const is_admin = getIsAdminFromJwt(token);
        const expire_timestamp = getExpireTimesamp(token);

        // Notify parent components of successful login
        emit("login-success", token, user_id.value, is_admin, expire_timestamp);
    } catch (err: any) {
        // Handle authentication errors
        error.value = "Login failed. Please try again.";
    }
}
</script>

<style scoped>
.login-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 1);
    display: flex;
    justify-content: center;
    align-items: center;
}

.login-modal {
    width: 22rem;
    margin-bottom: 5rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
