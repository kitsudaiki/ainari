<!-- 
// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//         http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License. 
-->

<template>
    <aside>
        <nav class="sidebar" role="navigation" aria-label="Main sidebar">
            <div v-for="menu in visibleMenus" :key="menu.name">
                <template v-if="menu.items && menu.items.length">
                    <div class="sidebar_drop">
                        <button
                            class="sidebar-btn dropdown-toggle"
                            :class="{ active: isOpen(menu.name) }"
                            :aria-expanded="isOpen(menu.name)"
                            @click="toggleDropdown(menu.name)"
                        >
                            <span>{{ menu.label }}</span>
                            <span
                                class="caret"
                                :class="{ open: isOpen(menu.name) }"
                                >▾</span
                            >
                        </button>

                        <div
                            class="sidebar_drop_content"
                            :ref="(el) => registerDropdownRef(menu.name, el)"
                            :style="{
                                maxHeight: dropdownHeights[menu.name] || '0px',
                            }"
                        >
                            <button
                                v-for="item in menu.items"
                                :key="item.view"
                                class="sidebar-btn sidebar_dropdown_entry"
                                :class="{ active: activeLocal === item.view }"
                                @click="select(item.view)"
                            >
                                {{ item.label }}
                            </button>
                        </div>
                    </div>
                </template>

                <template v-else>
                    <button
                        class="sidebar-btn"
                        :class="{ active: activeLocal === menu.name }"
                        @click="select(menu.name, { closeDropdowns: true })"
                    >
                        {{ menu.label }}
                    </button>
                </template>
            </div>
        </nav>
    </aside>
</template>

<script setup lang="ts">
import { ref, reactive, watch, nextTick, onMounted, computed } from "vue";

type MenuItem = { view: string; label: string };
type Menu = { name: string; label: string; items?: MenuItem[] };

interface Props {
    activeView?: string;
    isAdmin?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
    activeView: "Overview",
    isAdmin: false,
});

const emit = defineEmits<{
    (e: "change-view", view: string, id: string): void;
}>();

const visibleMenus = computed(() =>
    menus.value.filter(
        (menu) => menu.name !== "Admin" || props.isAdmin === true,
    ),
);

// Definitions of all items and subitems of the sidebar
const menus = ref<Menu[]>([
    { name: "Overview", label: "Overview" },
    {
        name: "Workload",
        label: "Workload",
        items: [{ view: "WorkloadCluster", label: "Cluster" }],
    },
    {
        name: "Storage",
        label: "Storage",
        items: [
            { view: "StorageDataset", label: "Dataset" },
            { view: "StorageCheckpoint", label: "Checkpoint" },
        ],
    },
    {
        name: "Admin",
        label: "Admin",
        items: [
            { view: "AdminUser", label: "User" },
            { view: "AdminProject", label: "Project" },
        ],
    },
]);

// pick first entry if no activeView is provided
const firstEntry = menus.value[0]?.name ?? "";
// Initialize active view with either the prop value or the first menu entry
const activeLocal = ref(props.activeView ?? firstEntry);
// Track which dropdowns are currently open
const openDropdowns = reactive<Record<string, boolean>>({});
// Store references to dropdown DOM elements
const dropdownRefs = ref<Record<string, HTMLElement | null>>({});
// Store calculated heights for dropdowns
const dropdownHeights = reactive<Record<string, string>>({});

/**
 * Check if a specific dropdown is currently open
 * @param name - The name of the dropdown to check
 * @returns boolean indicating whether the dropdown is open
 */
function isOpen(name: string) {
    return !!openDropdowns[name];
}

/**
 * Toggle the open state of a specific dropdown
 * @param name - The name of the dropdown to toggle
 */
function toggleDropdown(name: string) {
    openDropdowns[name] = !openDropdowns[name];
    updateHeights();
}

/**
 * Handle view selection and update dropdown states accordingly
 * @param view - The view to select
 * @param options - Optional configuration object
 * @param options.closeDropdowns - Whether to close all dropdowns after selection
 */
function select(view: string, options: { closeDropdowns?: boolean } = {}) {
    activeLocal.value = view;
    const id: string = "";
    emit("change-view", { view, id });

    // Always close all dropdowns first
    for (const k of Object.keys(openDropdowns)) {
        openDropdowns[k] = false;
    }

    if (options.closeDropdowns) {
        updateHeights();
        return;
    }

    // if selecting a subentry, open only its parent
    for (const menu of menus.value) {
        if (menu.items && menu.items.some((i) => i.view === view)) {
            openDropdowns[menu.name] = true;
        }
    }
    updateHeights();
}

/**
 * Register a dropdown reference for later height calculations
 * @param name - The name of the dropdown
 * @param el - The DOM element reference of the dropdown
 */
function registerDropdownRef(name: string, el: HTMLElement | null) {
    dropdownRefs.value[name] = el;
    if (!(name in openDropdowns)) openDropdowns[name] = false;
    nextTick(updateHeights);
}

/**
 * Update the heights of all dropdowns based on their open state
 * This is called after any state change that might affect dropdown visibility
 */
function updateHeights() {
    nextTick(() => {
        for (const name of Object.keys(dropdownRefs.value)) {
            const el = dropdownRefs.value[name];
            if (el) {
                dropdownHeights[name] = openDropdowns[name]
                    ? `${el.scrollHeight}px`
                    : "0px";
            } else {
                dropdownHeights[name] = "0px";
            }
        }
    });
}

// Initialize component state when mounted
onMounted(() => {
    // Set initial dropdown open states based on the active view
    for (const menu of menus.value) {
        openDropdowns[menu.name] = !!(
            menu.items && menu.items.some((i) => i.view === activeLocal.value)
        );
    }
    updateHeights();

    // load the overview-page after a login, even when the logout appeared on another page
    const view: string = "Overview";
    const id: string = "";
    emit("change-view", { view, id });
});

// Watch for changes to the active view and update dropdown states accordingly
watch(activeLocal, () => {
    for (const menu of menus.value) {
        if (
            menu.items &&
            menu.items.some((i) => i.view === activeLocal.value)
        ) {
            openDropdowns[menu.name] = true;
        }
    }
    updateHeights();
});

// Keep the local active view in sync with the prop value
watch(
    () => props.activeView,
    (v) => {
        if (v) activeLocal.value = v;
    },
);

// Watch for changes to admin status and update the Admin dropdown visibility
watch(
    () => {
        props.isAdmin;
    },
    (isAdmin) => {
        // Remove Admin dropdown if isAdmin is false
        if (!isAdmin) {
            openDropdowns["Admin"] = false;
            dropdownHeights["Admin"] = "0px";
        }
        updateHeights();
    },
);
</script>

<style scoped>
aside {
    height: 100vh;
    width: 13rem;
    margin-top: 1.5rem;
    box-shadow: var(--box-shadow-sidebar);
    background: var(--color-tile);
}

aside .sidebar {
    display: flex;
    flex-direction: column;
    position: relative;
    top: 0.5rem;
    padding: 0.25rem 0;
}

aside .sidebar button.sidebar-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    position: relative;
    height: 3rem;
    padding: 0 1rem;
    background-color: var(--color-tile);
    border: none;
    cursor: pointer;
    width: 100%;
    text-align: left;
    font: inherit;
    color: var(--color-text);
}

.caret {
    transition: transform 180ms ease;
}
.caret.open {
    transform: rotate(180deg);
}

.sidebar_drop_content {
    background-color: var(--color-shadow);
    overflow: hidden;
    transition: max-height 0.35s ease;
    max-height: 0;
}

aside .sidebar button.sidebar-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    position: relative;
    height: 3rem;
    padding: 0 1rem;
    background-color: var(--color-tile);
    border: none;
    cursor: pointer;
    width: 100%;
    text-align: left;
    font: inherit;
    color: var(--color-text);
}

/* highlight for both normal entries and dropdown parents */
aside .sidebar button.active {
    background-color: var(--color-highlight);
    color: invert(var(--color-text));
}

/* Subentries styled the same as top-level entries */
aside .sidebar button.sidebar_dropdown_entry {
    padding-left: 2rem;
    justify-content: flex-start;
    height: 2.5rem;
    background-color: var(--color-tile);
    color: var(--color-text);
}

/* highlight for subentries */
aside .sidebar button.sidebar_dropdown_entry.active {
    background-color: var(--color-highlight);
    color: invert(var(--color-text));
}
.sidebar-btn:hover,
.sidebar_dropdown_entry:hover {
    background-color: var(--color-highlight);
    color: invert(var(--color-text));
    transition: all 150ms;
}

.sidebar-btn:focus,
.sidebar_dropdown_entry:focus {
    outline: none;
}
</style>
