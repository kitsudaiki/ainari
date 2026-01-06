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
    <header class="topbar">
        <!-- <h1>Ainari Dashboard</h1> -->
        <h1></h1>

        <div class="profile-menu" @click.stop="toggleDropdown">
            <div class="avatar">
                {{ avatarLetter }}
            </div>
            <div class="topbar-dropdown" v-if="open">
                <button @click="logout">Logout</button>
            </div>
        </div>
    </header>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from "vue";

const emit = defineEmits<{ (e: "logout"): void }>();
const open = ref(false);
const props = defineProps<{ username: string | null }>();

const avatarLetter = computed(() => {
    return props.username ? props.username.charAt(0).toUpperCase() : "";
});

// toggle dropdown on avatar click
function toggleDropdown() {
    open.value = !open.value;
}

// logout
function logout() {
    emit("logout");
    open.value = false;
}

// close dropdown if clicked outside
function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    const menu = document.querySelector(".profile-menu");
    if (menu && !menu.contains(target)) {
        open.value = false;
    }
}

onMounted(() => {
    document.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    document.removeEventListener("click", handleClickOutside);
});

// hash string to HSL color
function stringToHslColor(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const h = hash % 360; // hue
    const s = 60; // saturation
    const l = 70; // lightness (pastel)
    return `hsl(${h}, ${s}%, ${l}%)`;
}
</script>

<style scoped>
.topbar {
    color: var(--color-text);
    background-color: var(--color-tile);
    box-shadow: var(--box-shadow-header);

    height: 4.2rem;
    width: 100%;
    left: 0rem;
    top: 0rem;
    min-height: 150;

    font-size: 1.2rem;
    font-weight: bold;

    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1rem;
    position: relative;
}

.profile-menu {
    position: relative;
    cursor: pointer;
    display: flex;
    align-items: center;
}

.avatar {
    width: 36px;
    height: 36px;
    /* border-radius: 50%; */
    /* border: 2px solid white; */
    background-color: var(--color-text);
    color: var(--color-tile);

    display: flex;
    align-items: center;
    justify-content: center;

    font-size: 24px;
    user-select: none;
}

.topbar-dropdown {
    position: absolute;
    top: 100%;
    right: 0;

    background: var(--color-tile);
    box-shadow: var(--box-shadow-header);
    border: 1px solid #ccc;

    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    min-width: 120px;
    z-index: 10;
}

.topbar-dropdown button {
    background: none;
    padding: 0.5rem;
    text-align: left;
    cursor: pointer;
    width: 100%;
}

.topbar-dropdown button:hover {
    background: var(--color-tile);
}
</style>
