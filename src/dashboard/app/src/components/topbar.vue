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
        <!-- slanted tab above the sidebar, which carries the brand -->
        <div class="brand">
            <img :src="logo" class="topbar-logo" alt="" />
            <span class="brand-name">Ainari</span>
        </div>

        <!-- project of the login, a click on it opens the project-switch -->
        <button
            v-if="projectId"
            class="project-chip"
            title="Switch project"
            @click="switchProject"
        >
            <span class="project-chip-caption">Project</span>
            <span class="project-chip-value">{{ projectId }}</span>
        </button>

        <div class="topbar-spacer"></div>

        <div class="clock" title="Current local time">
            <span class="clock-date">{{ clockDate }}</span>
            <span class="clock-time">{{ clockTime }}</span>
            <span class="clock-zone">{{ clockZone }}</span>
        </div>

        <div class="profile-menu" @click.stop="toggleDropdown">
            <span class="profile-name">{{ username }}</span>
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
const props = defineProps<{ username: string | null; projectId?: string | null }>();

// current local time, which is updated every second
const now = ref(new Date());
var clockInterval: number | undefined;
const pad = (value: number) => String(value).padStart(2, "0");
const clockDate = computed(
    () =>
        `${now.value.getFullYear()}-${pad(now.value.getMonth() + 1)}-${pad(now.value.getDate())}`,
);
const clockTime = computed(
    () =>
        `${pad(now.value.getHours())}:${pad(now.value.getMinutes())}:${pad(now.value.getSeconds())}`,
);
// short name of the local time-zone, like "CEST", which also follows the daylight saving time
const clockZone = computed(
    () =>
        new Intl.DateTimeFormat(undefined, { timeZoneName: "short" })
            .formatToParts(now.value)
            .find((part) => part.type === "timeZoneName")?.value ?? "",
);

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
    clockInterval = window.setInterval(() => (now.value = new Date()), 1000);
});

onBeforeUnmount(() => {
    document.removeEventListener("click", handleClickOutside);
    clearInterval(clockInterval);
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
    background: linear-gradient(
        180deg,
        rgba(12, 17, 23, 0.96) 0%,
        rgba(9, 13, 18, 0.9) 100%
    );
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-bottom: 1px solid var(--color-border-soft);
    box-shadow: var(--box-shadow-header);
    /* The topbar is its own stacking-context, so the z-index of the avatar-dropdown only counts
    within it. It has to be above all elements of the page (add-buttons, table-dropdowns, ...),
    but below the modals (1000) and the error-popups (9999). */
    z-index: 100;

    height: 3.6rem;
    width: 100%;
    flex-shrink: 0;

    display: flex;
    align-items: stretch;
    gap: 1.25rem;
    padding-right: 1rem;
    position: relative;
}

/* The tab has the width of the sidebar and a line at its right edge, which continues the
border of the sidebar. */
.brand {
    position: relative;
    width: 15rem;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0 1.1rem;
    border-right: 1px solid var(--color-border-soft);
}

.topbar-logo {
    height: 2.2rem;
    width: auto;
    user-select: none;
    filter: drop-shadow(0 0 0.5rem var(--color-highlight-glow));
}

/* wordmark: wide letter-spacing, like the name on the hull of a ship */
.brand-name {
    font-family: var(--title-font-family);
    font-size: 1.25rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.42em;
    color: var(--color-text);
}

.project-chip {
    align-self: center;
    display: flex;
    align-items: stretch;
    height: 2rem;
    --notch-color: var(--color-border);
    background: var(--corner-notch), rgba(255, 255, 255, 0.03);
    border: 1px solid var(--color-border-soft);
    cursor: pointer;
    font-family: var(--title-font-family);
    transition:
        border-color 150ms,
        background-color 150ms;
}

.project-chip:hover {
    --notch-color: var(--color-highlight);
    border-color: var(--color-highlight);
    background: var(--corner-notch), var(--color-highlight-faint);
    filter: none;
}

.project-chip-caption {
    display: flex;
    align-items: center;
    padding: 0 0.6rem;
    background: rgba(255, 255, 255, 0.05);
    border-right: 1px solid var(--color-border-soft);
    color: var(--color-text-light);
    font-size: 0.8rem;
    letter-spacing: 0.08em;
}

.project-chip-value {
    display: flex;
    align-items: center;
    padding: 0 1.1rem 0 0.7rem;
    color: var(--color-text);
    font-size: 0.95rem;
    font-weight: 500;
    letter-spacing: 0.04em;
}

.topbar-spacer {
    flex: 1;
}

.clock {
    align-self: center;
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    font-family: var(--title-font-family);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-light);
}

.clock-date {
    font-size: 0.85rem;
}

.clock-time {
    font-size: 1.15rem;
    font-weight: 500;
    letter-spacing: 0.06em;
    color: var(--color-text);
}

.clock-zone {
    font-size: 0.75rem;
    color: var(--color-highlight);
    letter-spacing: 0.1em;
}

.profile-menu {
    position: relative;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding-left: 1.25rem;
    border-left: 1px solid var(--color-border-soft);
}

.profile-name {
    font-family: var(--title-font-family);
    font-size: 1rem;
    color: var(--color-text);
    letter-spacing: 0.03em;
}

.avatar {
    width: 2.1rem;
    height: 2.1rem;
    background: linear-gradient(
        180deg,
        var(--color-highlight) 0%,
        var(--color-highlight-dark) 100%
    );
    color: var(--color-on-highlight);
    clip-path: var(--chamfer-sm-both);

    display: flex;
    align-items: center;
    justify-content: center;

    font-family: var(--title-font-family);
    font-size: 1.15rem;
    font-weight: 600;
    line-height: 1;
    user-select: none;
    transition: filter 150ms;
}

.profile-menu:hover .avatar {
    filter: brightness(115%);
}

.topbar-dropdown {
    position: absolute;
    top: calc(100% + 0.45rem);
    right: 0;

    background: var(--glass-sheen), rgba(12, 17, 23, 0.97);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    box-shadow: var(--box-shadow-header);
    border: 1px solid var(--color-border);
    border-top: 2px solid var(--color-highlight);

    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    min-width: 12rem;
    z-index: 10;
}

.topbar-dropdown button {
    background: none;
    padding: 0.45rem 0.7rem;
    text-align: left;
    cursor: pointer;
    width: 100%;
    font-size: 0.95rem;
    white-space: nowrap;
}

.topbar-dropdown button:hover {
    background: var(--color-highlight);
    color: var(--color-on-highlight);
    filter: none;
}
</style>
