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
                        :class="{ invalid: userIdError }"
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
                        :class="{ invalid: passwordError }"
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

import context from "../auth_context";
import { getConfig } from "../config";

// Reactive references to store user input and error messages
const user_id = ref("");
const password = ref("");
const error = ref("");

const userIdError = ref(false);
const passwordError = ref(false);

// Define custom events that this component can emit
// This emits a "login-success" event when login is successful, passing the token and user ID
const emit = defineEmits<{
    (e: "login-success", token: string, userId: string): void;
}>();

/**
 * Attempts to authenticate the user with the provided credentials
 * and establish an authenticated session.
 */
async function login() {
    try {
        userIdError.value = user_id.value.length < 4;
        passwordError.value = password.value.length < 8;

        if (userIdError.value || passwordError.value) {
            return; // stop login
        }
        error.value = "";

        // Prepare URL-encoded form data for the authentication request
        // This is the standard format expected by OAuth 2.0 token endpoints
        const params = new URLSearchParams();
        params.append("grant_type", "client_credentials"); // Specifies the OAuth 2.0 flow
        params.append("token_format", "jwt"); // Requests a JSON Web Token
        params.append("client_id", user_id.value); // Client identifier from user input
        params.append("client_secret", password.value); // Client secret from user input

        // Loac miko-address from the config-file for the initial connection
        // The addresses of all the other components will be requested from Miko
        const { apiUrl } = getConfig();
        const miko_api = axios.create({
            baseURL: apiUrl,
        });

        // Send the authentication request to the API endpoint
        // The endpoint expects form-encoded data with specific headers
        const login_resp = await miko_api.post("/v1alpha/token", params, {
            headers: {
                "Content-Type": "application/x-www-form-urlencoded", // Required header for form data
            },
        });

        // Log the response for debugging purposes
        // This helps with troubleshooting authentication issues
        // console.log("Login response:", login_resp);

        const token = login_resp.data.access_token;

        // Store the authentication token in the global context
        // This makes the token available to other components in the application
        await context.createAuthContext(token);
        const is_admin = context.getIsAdminFromJwt(token);
        const expire_timestamp = context.getExpireTimesamp(token);

        // Emit a success event to notify parent components
        // This allows the parent to handle post-login actions
        emit("login-success", token, user_id.value, is_admin, expire_timestamp);
    } catch (err: any) {
        error.value = "Login failed. Please try again.";
        console.error("Login error:", err);
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
.invalid {
    border: 2px solid #ff4d4f;
}
</style>
