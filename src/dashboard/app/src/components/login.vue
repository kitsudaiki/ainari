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
        <div class="login-brand">
            <img :src="logo" class="login-logo" alt="" />
            <div class="login-wordmark">Ainari</div>
        </div>

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
                        @keyup.enter="login"
                        :class="{ invalid_input: userIdError }"
                    />
                    <p v-if="userIdError" class="error-msg">
                        User-ID must be at least 4 characters
                    </p>
                </div>

                <div class="login-field-spacing">
                    <input
                        v-model="password"
                        type="password"
                        id="login_pw_field"
                        placeholder="Password"
                        @keyup.enter="login"
                        :class="{ invalid_input: passwordError }"
                    />
                    <p v-if="passwordError" class="error-msg">
                        Password must be at least 8 characters
                    </p>
                </div>

                <div class="login-field-spacing">
                    <input
                        v-model="project_id"
                        type="text"
                        id="login_project_field"
                        placeholder="Project-ID (optional)"
                        @keyup.enter="login"
                        :class="{ invalid_input: projectIdError }"
                    />
                    <p v-if="projectIdError" class="error-msg">
                        Project-ID must be between 4 and 127 characters
                    </p>
                </div>

                <!-- Rejected login, for example an invalid project-id -->
                <p v-if="error" class="error-msg">{{ error }}</p>
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

import { handleAxiosError, responseMessage } from "@/handleAxiosError";

import {
    createAuthContext,
    getIsAdminFromJwt,
    getExpireTimesamp,
} from "@/auth_context";
import { getConfig } from "@/config";

const logo = new URL("../assets/ainari_logo_mini_transparent.png", import.meta.url).href;

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
// optional, if empty, the backend uses the default-project of the user
const project_id = ref("");
const error = ref("");
const userIdError = ref(false);
const passwordError = ref(false);
const projectIdError = ref(false);

/**
 * Validates the user input before attempting login
 * @returns boolean - true if validation passes, false otherwise
 */
function validateInput(): boolean {
    userIdError.value = user_id.value.length < 4;
    passwordError.value = password.value.length < 8;
    // the project-id is optional, so only a given one is checked
    const projectId = project_id.value.trim();
    projectIdError.value =
        projectId.length > 0 && (projectId.length < 4 || projectId.length > 127);
    return !(userIdError.value || passwordError.value || projectIdError.value);
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
        // without a project-id, the backend creates the token for the default-project
        const projectId = project_id.value.trim();
        if (projectId.length > 0) {
            params.append("project_id", projectId);
        }

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
        // Wrong credentials and a project, which doesn't exist or to which the user is not
        // assigned, are all rejected with 401. The reason is given by the backend.
        if (axios.isAxiosError(err) && err.response?.status === 401) {
            const reason = responseMessage(err.response.data) ?? "Login failed";
            error.value = `Unauthorized: ${reason}`;
        } else {
            error.value = handleAxiosError(err, "Login failed");
        }
    }
}
</script>

<style scoped>
.login-overlay {
    position: fixed;
    inset: 0;
    background: radial-gradient(
        ellipse 60% 50% at 50% 30%,
        rgb(from var(--color-highlight) r g b / 0.08) 0%,
        transparent 70%
    );
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 2rem;
    padding-bottom: 5rem;
}

/* logo with the glow of a fox-fire and the name below it */
.login-brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.9rem;
    animation: panel-in 600ms ease-out backwards;
}

.login-logo {
    height: 4.5rem;
    width: auto;
    filter: drop-shadow(0 0 1.2rem var(--color-highlight-glow));
}

.login-wordmark {
    font-family: var(--title-font-family);
    font-size: 1.6rem;
    font-weight: 500;
    text-transform: uppercase;
    /* the letter-spacing is also added after the last letter, so it is compensated by the
    padding on the left, to keep the word centered */
    letter-spacing: 0.6em;
    padding-left: 0.6em;
    color: var(--color-text);
}

.login-modal {
    width: 24rem;
    animation: panel-in 400ms ease-out 150ms backwards;
}

.login-field-spacing {
    margin-top: 0.5rem;
}
</style>
