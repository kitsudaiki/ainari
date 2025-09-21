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
    <div class="app">
        <div class="background"></div>
        <!-- blur overlay -->
        <div class="overlay"></div>

        <!-- Login popup -->
        <Login v-if="!token" @login-success="handleLoginSuccess" />

        <!-- Dashboard -->
        <template v-else>
            <Topbar :username="username" @logout="handleLogout" />
            <div class="main">
                <Sidebar
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
</template>

<script setup lang="ts">
import { ref, provide } from "vue";
import Sidebar from "./components/sidebar.vue";
import Topbar from "./components/topbar.vue";
import Login from "./components/login.vue";
import Overview from "./components/overview.vue";
import AdminUser from "./components/admin/user.vue";
import AdminProject from "./components/admin/project.vue";
import StorageCheckpoint from "./components/storage/checkpoint.vue";
import StorageDataset from "./components/storage/dataset.vue";
import WorkloadCluster from "./components/workload/cluster.vue";
import WorkloadTask from "./components/workload/task.vue";

import "./styles/base.css";
import "./styles/other.css";
import "./styles/card.css";
import "./styles/modal.css";
import "./styles/dropdown.css";
import "./styles/button.css";
import "./styles/table.css";
import "./styles/tab.css";
import "./styles/primevue_overrides.css";

const currentView = ref("Overview");
const currentId = ref<string | null>(null);

const token = ref<string | null>(localStorage.getItem("jwtToken"));
const username = ref<string | null>(localStorage.getItem("username")); // store username
const components = {
    Overview,
    AdminUser,
    AdminProject,
    StorageCheckpoint,
    StorageDataset,
    WorkloadCluster,
    WorkloadTask,
};
const acceptIcon = new URL("./assets/accept.svg", import.meta.url).href;
const cancelIcon = new URL("./assets/close.svg", import.meta.url).href;

provide("icons", { acceptIcon, cancelIcon });

function handleLoginSuccess(newToken: string, user: string) {
    localStorage.setItem("jwtToken", newToken);
    localStorage.setItem("username", user);
    token.value = newToken;
    username.value = user;
}

function handleLogout() {
    localStorage.removeItem("jwtToken");
    localStorage.removeItem("username");
    token.value = null;
    username.value = null;
}
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
    padding: 1rem;
    overflow-y: auto;
}
</style>
