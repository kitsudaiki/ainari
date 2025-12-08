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
                        name="id"
                        placeholder="User-ID"
                        required
                    />
                </div>
                <br /><br />
                <div>
                    <input
                        v-model="password"
                        type="password"
                        id="login_pw_field"
                        name="password"
                        placeholder="Password"
                        required
                    />
                </div>
            </div>

            <!-- Modal bottombar -->
            <div class="modal-bottombar">
                <button @click="login">Login</button>
                <p v-if="error" class="error-msg">{{ error }}</p>
            </div>
        </div>
    </div>
 
    <!--<div v-if="error" class="error-popup">
        <button class="error-close-btn" @click="error = ''">✕</button>
        {{ error }}
    </div> -->
</template>

<script setup lang="ts">
import { ref } from "vue";
import api from "../api";
import context from "../auth_context";

// Reactive references to store user input and error messages
const user_id = ref("");
const password = ref("");
const error = ref("");

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
        // Reset any previous error messages
        error.value = "";

        // Prepare URL-encoded form data for the authentication request
        // This is the standard format expected by OAuth 2.0 token endpoints
        const params = new URLSearchParams();
        params.append("grant_type", "client_credentials"); // Specifies the OAuth 2.0 flow
        params.append("token_format", "jwt"); // Requests a JSON Web Token
        params.append("client_id", user_id.value); // Client identifier from user input
        params.append("client_secret", password.value); // Client secret from user input

        // Send the authentication request to the API endpoint
        // The endpoint expects form-encoded data with specific headers
        const login_resp = await api.miko_api.post("/v1alpha/token", params, {
            headers: {
                "Content-Type": "application/x-www-form-urlencoded", // Required header for form data
            },
        });

        // Log the response for debugging purposes
        // This helps with troubleshooting authentication issues
        // console.log("Login response:", login_resp);

        // Extract the access token from the response
        // This token will be used for authenticated API requests
        const token = login_resp.data.access_token;

        // Store the authentication token in the global context
        // This makes the token available to other components in the application
        await context.createAuthContext(token);

        // Emit a success event to notify parent components
        // This allows the parent to handle post-login actions
        emit("login-success", token, user_id.value);
    } catch (err: any) {
        // Handle any errors that occur during the login process
        // This provides user-friendly feedback when authentication fails
        error.value = "Login failed. Please try again.";

        // In a production environment, you might want to log more detailed error information
        // for debugging purposes, but keep sensitive information out of user-facing messages
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
}

.login-modal {
    height: 18rem;
    width: 20rem;
}
</style>
