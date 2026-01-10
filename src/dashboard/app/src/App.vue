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
    <div class="app">
        <div class="background">
            <img
                src="./assets/background_pattern.svg"
                class="logo-icon top-left"
            />
            <img
                src="./assets/background_pattern2.svg"
                class="logo-icon bottom-right"
            />
        </div>
        <!-- blur overlay -->
        <div class="overlay"></div>

        <!-- Login popup -->
        <Login v-if="!isLoggedIn" @login-success="handleLoginSuccess" />

        <!-- Dashboard -->
        <template v-else>
            <Topbar :username="username" @logout="handleLogout" />
            <div class="main">
                <Sidebar
                    :isAdmin="isAdmin"
                    @change-view="
                        ({ view, id }) => {
                            currentView = view;
                            currentId = id;
                        }
                    "
                />
                <div class="content">
                    <!-- @change-view="currentView = $event" right here provides each loaded 
                     component the ability to change the component, like the sidebar does -->
                    <component
                        :is="components[currentView]"
                        :id="currentId"
                        @change-view="
                            ({ view, id }) => {
                                currentView = view;
                                currentId = id ?? null;
                            }
                        "
                    />
                </div>
            </div>
        </template>
    </div>

    <div v-if="tokenExpireError" class="error-popup">
        <button class="error-close-btn" @click="tokenExpireError = ''">
            ✕
        </button>
        {{ tokenExpireError }}
    </div>
</template>

<script setup lang="ts">
// Import necessary Vue composition API functions
import { ref, provide, onMounted, onUnmounted } from "vue";

// Import all the Vue components used in the application
import Sidebar from "@/components/sidebar.vue";
import Topbar from "@/components/topbar.vue";
import Login from "@/components/login.vue";
import Overview from "@/components/overview.vue";
import AdminUser from "@/components/admin/user/user_overview.vue";
import AdminProject from "@/components/admin/project/project_overview.vue";
import StorageCheckpoint from "@/components/storage/checkpoint/checkpoint_overview.vue";
import StorageDataset from "@/components/storage/dataset/dataset_overview.vue";
import WorkloadCluster from "@/components/workload/cluster/cluster_overview.vue";
import WorkloadTask from "@/components/workload/task/task_overview.vue";
import { getAuthContext } from "@/auth_context";

// Import all CSS styles for the application
import "@/styles/base.css";
import "@/styles/other.css";
import "@/styles/card.css";
import "@/styles/modal.css";
import "@/styles/dropdown.css";
import "@/styles/button.css";
import "@/styles/table.css";
import "@/styles/tab.css";
import "@/styles/devider.css";
import "@/styles/primevue_overrides.css";

// Reactive reference to track the current active view/component
// Defaults to "Overview" when the application loads
const currentView = ref("Overview");
const currentId = ref<string | null>(null);
const isLoggedIn = ref<boolean>(!!localStorage.getItem("ainari_authContext"));
const username = ref<string | null>(localStorage.getItem("username"));
const isAdmin = ref<boolean>(getAuthContext().is_admin === "true");
const tokenExpireError = ref<string>("");
var expiryInterval: number | undefined;

// Object containing all the available view components
// These will be dynamically rendered based on the currentView value
const components = {
    Overview,
    AdminUser,
    AdminProject,
    StorageCheckpoint,
    StorageDataset,
    WorkloadCluster,
    WorkloadTask,
};

// URLs for the accept and cancel icons used throughout the application
// These are imported as module URLs and converted to absolute URLs
const acceptIcon = new URL("./assets/accept.svg", import.meta.url).href;
const cancelIcon = new URL("./assets/close.svg", import.meta.url).href;

// Provide the icons to all child components via Vue's provide/inject system
// This makes the icons available throughout the component tree without prop drilling
provide("icons", { acceptIcon, cancelIcon });

/**
 * Handles successful login by updating the application state
 * @param newToken - The new authentication token received from the login process
 * @param user - The username of the logged-in user
 * @param is_admin - True, if the user is an admin
 */
function handleLoginSuccess(
    newToken: string,
    user: string,
    is_admin: string,
    expire_timestamp: number,
) {
    // Store the username in localStorage for persistence across page reloads
    localStorage.setItem("username", user);
    isLoggedIn.value = true;
    username.value = user;
    isAdmin.value = is_admin === "true";
    tokenExpireError.value = "";

    // Start watcher to check if the token is expired. To avoid conflicts
    // the logout will be done 30 seonds before the token really expire.
    startTokenExpiryWatcher(expire_timestamp - 30, handleLogout);

    // Update the login state to true to disable the login-modal
}

/**
 * Handles user logout by clearing the authentication state
 * This removes all user-related data from localStorage and updates the reactive state
 */
function handleLogout() {
    // Remove the authentication token and username from localStorage
    localStorage.removeItem("ainari_authContext");
    localStorage.removeItem("username");

    // Update the reactive state to reflect the logged-out status
    isLoggedIn.value = false;
    username.value = null;
}

function startTokenExpiryWatcher(
    expireTimeUnix: number,
    handleLogout: () => void,
) {
    // Clear any previous watcher (important!)
    if (expiryInterval) {
        clearInterval(expiryInterval);
    }

    expiryInterval = window.setInterval(() => {
        const nowUnix = Math.floor(Date.now() / 1000);

        // Handle expired token
        if (nowUnix >= expireTimeUnix) {
            tokenExpireError.value = "Token expired. Please login again.";
            clearInterval(expiryInterval);
            expiryInterval = undefined;
            handleLogout();
        }
    }, 2000); // check token every 2 seconds
}

onMounted(() => {
    if (isLoggedIn) {
        const expire_timestamp = getAuthContext().expire_timestamp;

        // In case there is no timestamp, something is wrong and you definitelly
        // has to login again.
        if (expire_timestamp === null) {
            handleLogout();
            return;
        }

        startTokenExpiryWatcher(expire_timestamp, handleLogout);
    }
});

onUnmounted(() => {
    if (expiryInterval) {
        clearInterval(expiryInterval);
    }
});
</script>

<style scoped>
.app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    position: relative;
    z-index: 0;
}

.background {
    background-color: var(--color-background);
    /* background-picture
    background: url("./src/assets/background.jpg") no-repeat
        center center fixed; */
    background-size: cover;
    position: fixed;
    inset: 0;
    z-index: -2;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh; /* THIS is the important part */
}

.overlay {
    position: fixed;
    inset: 0;
    /* blur-effect
    backdrop-filter: blur(20px);
    background-color: rgba(59, 63, 66, 0.5); */
    z-index: -1;
}

.main {
    flex: 1;
    display: flex;
    overflow: hidden;
}

.content {
    flex: 1;
    margin: 1rem;
    overflow-y: auto;
}

.logo-icon {
    height: auto;
    filter: invert(1);
    position: absolute;
    opacity: 0.025;
}

.top-left {
    width: 50vw;

    top: 0;
    left: 0;
}

.bottom-right {
    width: 28vw;

    bottom: 0;
    right: 0;
}
</style>
