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
</template>
<script setup lang="ts">
import { ref } from "vue";
import api from "../api";

const user_id = ref("");
const password = ref("");
const error = ref("");

const emit = defineEmits<{ (e: "login-success", token: string): void }>();

async function login() {
    try {
        error.value = "";

        // prepare URL-encoded form data
        const params = new URLSearchParams();
        params.append("grant_type", "client_credentials");
        params.append("token_format", "jwt");
        params.append("client_id", user_id.value);
        params.append("client_secret", password.value);

        const response = await api.torii_api.post("/v1alpha/token", params, {
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
            },
        });
        // debug-output
        // console.log("Login response:", response);

        const token = response.data.access_token;
        emit("login-success", token, user_id.value);
    } catch (err: any) {
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
}

.login-modal {
    height: 18rem;
    width: 20rem;
}
</style>
