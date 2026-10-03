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
        <img :src="logo" class="topbar-logo" alt="Ainari" />

        <div class="profile-menu" @click.stop="toggleDropdown">
            <div class="avatar">
                {{ avatarLetter }}
            </div>
            <div class="topbar-dropdown" v-if="open">
                <button @click="switchProject">Switch Project</button>
                <button @click="changePassphrase">Change Passphrase</button>
                <button @click="logout">Logout</button>
            </div>
        </div>
    </header>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from "vue";

const logo = new URL("../assets/ainari_logo_mini_transparent.png", import.meta.url).href;

const emit = defineEmits<{
    (e: "logout"): void;
    (e: "switch-project"): void;
    (e: "change-passphrase"): void;
}>();
const open = ref(false);
const props = defineProps<{ username: string | null }>();

const avatarLetter = computed(() => {
    return props.username ? props.username.charAt(0).toUpperCase() : "";
});
// toggle dropdown on avatar click
function toggleDropdown() {
    open.value = !open.value;
}

// open the modal to switch into another project of the user
function switchProject() {
    emit("switch-project");
    open.value = false;
}

// open the modal to change the passphrase of the own user
function changePassphrase() {
    emit("change-passphrase");
    open.value = false;
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
    background:
        linear-gradient(
            90deg,
            rgba(22, 24, 27, 0.85) 30%,
            rgba(22, 24, 27, 0.35) 100%
        ),
        var(--hex-pattern),
        rgba(22, 24, 27, 0.4);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-bottom: 1px solid var(--color-border);
    box-shadow: var(--box-shadow-header);
    /* The topbar is its own stacking-context, so the z-index of the avatar-dropdown only counts
    within it. It has to be above all elements of the page (add-buttons, table-dropdowns, ...),
    but below the modals (1000) and the error-popups (9999). */
    z-index: 100;

    height: 4.2rem;
    width: 100%;
    left: 0rem;
    top: 0rem;
    min-height: 150;

    font-size: 1.2rem;

    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1rem;
    position: relative;
}

.topbar-logo {
    height: 2.8rem;
    width: auto;
    user-select: none;
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
    background-color: var(--color-selected);
    color: var(--color-text-dark);
    border: 1px solid var(--color-border);
    box-shadow: inset 0 -0.2rem 0 var(--color-highlight);

    display: flex;
    align-items: center;
    justify-content: center;

    font-size: 24px;
    line-height: 1;
    /* Centers the capital letter in the light area above the accent-bar:
    0.1em, because the capitals of Barlow (cap-height 0.7em, ascent 1.0em,
    descent 0.2em) sit 0.05em below the middle of the text-box, and 0.2rem for the
    accent-bar at the bottom. Padding shifts the flex-center by half its size. */
    padding-bottom: calc(0.1em + 0.2rem);
    user-select: none;
}

.topbar-dropdown {
    position: absolute;
    top: 100%;
    right: 0;

    background: var(--glass-sheen), var(--glass-bg-strong);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    box-shadow: var(--box-shadow-header);
    border: 1px solid var(--color-border);

    padding: 0.25rem;
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
    background: var(--color-highlight);
    color: var(--color-on-highlight);
    filter: none;
}
</style>
