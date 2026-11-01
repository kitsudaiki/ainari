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
                            <span class="entry-label">
                                <i :class="['pi', menu.icon]"></i>
                                {{ menu.label }}
                            </span>
                            <span
                                class="caret"
                                :class="{ open: isOpen(menu.name) }"
                            ></span>
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
                                <span class="entry-label">
                                    <i :class="['pi', item.icon]"></i>
                                    {{ item.label }}
                                </span>
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
                        <span class="entry-label">
                            <i :class="['pi', menu.icon]"></i>
                            {{ menu.label }}
                        </span>
                    </button>
                </template>
            </div>
        </nav>

        <!-- project of the login, a click on it opens the project-switch -->
        <div v-if="projectId" class="sidebar-project">
            <button
                class="project-chip"
                :title="`Project ${projectId}, click to switch the project`"
                @click="emit('switch-project')"
            >
                <span class="project-chip-caption">Project</span>
                <span class="project-chip-value">{{ projectId }}</span>
            </button>
        </div>

        <footer class="sidebar-footer">
            <span>Ainari</span>
            <span class="sidebar-version">v{{ version }}</span>
        </footer>
    </aside>
</template>

<script setup lang="ts">
import { ref, reactive, watch, nextTick, onMounted, computed } from "vue";
import { version } from "../../package.json";

// icon is the class of a primeicons-icon, like "pi-home", or of a custom icon defined in the
// style below, like "pi-disc"
type MenuItem = { view: string; label: string; icon: string };
type Menu = { name: string; label: string; icon: string; items?: MenuItem[] };

interface Props {
    activeView?: string;
    isAdmin?: boolean;
    /** project, for which the user is logged in */
    projectId?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
    activeView: "Overview",
    isAdmin: false,
    projectId: null,
});

const emit = defineEmits<{
    (e: "change-view", view: string, id: string): void;
    (e: "switch-project"): void;
}>();

const visibleMenus = computed(() =>
    menus.value.filter(
        (menu) => menu.name !== "Admin" || props.isAdmin === true,
    ),
);

// Definitions of all items and subitems of the sidebar
const menus = ref<Menu[]>([
    { name: "Overview", label: "Overview", icon: "pi-home" },
    {
        name: "Workload",
        label: "Workload",
        icon: "pi-bolt",
        items: [
            {
                view: "WorkloadVirtualMachine",
                label: "Virtual Machines",
                icon: "pi-desktop",
            },
        ],
    },
    {
        name: "Storage",
        label: "Storage",
        icon: "pi-database",
        items: [
            { view: "StorageImage", label: "Image", icon: "pi-disc" },
            { view: "StorageBackup", label: "Backups", icon: "pi-save" },
        ],
    },
    {
        name: "Network",
        label: "Network",
        icon: "pi-sitemap",
        items: [
            { view: "NetworkNetwork", label: "Networks", icon: "pi-share-alt" },
            {
                view: "NetworkFloatingIp",
                label: "Floating IPs",
                icon: "pi-globe",
            },
        ],
    },
    {
        name: "Security",
        label: "Security",
        icon: "pi-shield",
        items: [
            { view: "SecuritySecret", label: "Secrets", icon: "pi-lock" },
            { view: "SecurityPublicKey", label: "Public Keys", icon: "pi-key" },
        ],
    },
    {
        name: "Admin",
        label: "Admin",
        icon: "pi-cog",
        items: [
            { view: "AdminUser", label: "User", icon: "pi-user" },
            { view: "AdminProject", label: "Project", icon: "pi-briefcase" },
            { view: "AdminQuota", label: "Quota", icon: "pi-chart-pie" },
            { view: "AdminHost", label: "Host", icon: "pi-server" },
            { view: "AdminVmType", label: "VM Types", icon: "pi-sliders-h" },
            {
                view: "AdminErrorLog",
                label: "Error Log",
                icon: "pi-exclamation-triangle",
            },
            { view: "AdminAuditLog", label: "Audit Log", icon: "pi-history" },
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
    height: 100%;
    width: 15rem;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    padding: 1.25rem 0 0 0;
    background: rgba(9, 13, 18, 0.86);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-right: 1px solid var(--color-border-soft);
    overflow-y: auto;
}

aside .sidebar {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0 0.75rem;
}

/* top-level entries are flat rows, which light up on hover */
aside .sidebar button.sidebar-btn {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    position: relative;
    height: 2.75rem;
    padding: 0 0.8rem 0 0.9rem;
    background: transparent;
    clip-path: var(--chamfer-sm);
    cursor: pointer;
    width: 100%;
    text-align: left;
    font: inherit;
    font-family: var(--title-font-family);
    font-weight: 500;
    font-size: 0.92rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--color-text-light);
    transition:
        background-color 150ms,
        color 150ms;
}

aside .sidebar button.sidebar-btn:hover {
    background-color: rgba(255, 255, 255, 0.04);
    color: var(--color-text);
    filter: none;
}

/* icon and text of an entry */
.entry-label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.8rem;
}

.entry-label .pi {
    width: 1.2rem;
    text-align: center;
    font-size: 1rem;
    transition: color 150ms;
}

/* primeicons has no CD/DVD, so it is drawn by a mask in the current text-color. This
way it changes the color on hover and selection like the other icons. The disc is a
little bit bigger, because its round shape looks smaller than the other icons. */
.entry-label .pi-disc::before {
    content: "";
    display: inline-block;
    width: 1.2em;
    height: 1.2em;
    vertical-align: middle;
    background-color: currentColor;
    mask: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round'%3E%3Ccircle cx='12' cy='12' r='9.5'/%3E%3Ccircle cx='12' cy='12' r='2.5'/%3E%3Cpath d='M12 5.5a6.5 6.5 0 0 1 6.5 6.5'/%3E%3C/svg%3E")
        center / contain no-repeat;
}

/* caret: a small chevron, which points down when the section is open */
.caret {
    width: 0.4rem;
    height: 0.4rem;
    border-right: 1.5px solid currentColor;
    border-bottom: 1.5px solid currentColor;
    transform: translateY(-25%) rotate(45deg);
    transition: transform 180ms ease;
    opacity: 0.7;
}
.caret.open {
    transform: translateY(15%) rotate(-135deg);
}

/* open section: the text and the icon light up */
aside .sidebar button.dropdown-toggle.active {
    color: var(--color-text);
}

aside .sidebar button.dropdown-toggle.active .pi {
    color: var(--color-highlight);
}

/* selected top-level entry: tinted row with a glowing bar on the left */
aside .sidebar button.sidebar-btn.active:not(.dropdown-toggle) {
    background: linear-gradient(
        90deg,
        rgb(from var(--color-highlight) r g b / 0.2) 0%,
        rgb(from var(--color-highlight) r g b / 0.03) 100%
    );
    color: var(--color-text);
    box-shadow: inset 2px 0 0 var(--color-highlight);
}

aside .sidebar button.sidebar-btn.active:not(.dropdown-toggle) .pi {
    color: var(--color-highlight);
}

/* sub-entries hang on a vertical line, like the branches of a tree */
.sidebar_drop_content {
    display: flex;
    flex-direction: column;
    margin-left: 1.5rem;
    border-left: 1px solid var(--color-border-soft);
    overflow: hidden;
    transition: max-height 0.35s ease;
    max-height: 0;
}

.sidebar_drop_content > :first-child {
    margin-top: 0.2rem;
}

.sidebar_drop_content > :last-child {
    margin-bottom: 0.4rem;
}

aside .sidebar button.sidebar_dropdown_entry {
    justify-content: flex-start;
    height: 2.3rem;
    /* without this, the flex-container shrinks the entries to their text, while it is
    collapsed, and the opened height is then measured with these shrunk entries */
    flex-shrink: 0;
    padding-left: 1.1rem;
    font-family: var(--text-font-family);
    font-size: 0.92rem;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0.01em;
    clip-path: none;
}

/* branch from the line of the tree to the entry */
aside .sidebar button.sidebar_dropdown_entry::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    width: 0.55rem;
    height: 1px;
    background: var(--color-border);
}

aside .sidebar button.sidebar_dropdown_entry .pi {
    font-size: 0.85rem;
}

/* selected sub-entry: the branch and the line beside it glow in the highlight-color */
aside .sidebar button.sidebar_dropdown_entry.active {
    background: linear-gradient(
        90deg,
        rgb(from var(--color-highlight) r g b / 0.18) 0%,
        transparent 100%
    );
    color: var(--color-text);
    box-shadow: inset 2px 0 0 var(--color-highlight);
}

aside .sidebar button.sidebar_dropdown_entry.active::before {
    background: var(--color-highlight);
    box-shadow: 0 0 0.4rem var(--color-highlight);
}

aside .sidebar button.sidebar_dropdown_entry.active .pi {
    color: var(--color-highlight);
}

.sidebar-btn:focus,
.sidebar_dropdown_entry:focus {
    outline: none;
}

.sidebar-btn:focus-visible {
    outline: 1px solid var(--color-highlight);
    outline-offset: -1px;
}

.sidebar-project {
    padding: 0 0.75rem 0.75rem 0.75rem;
}

.project-chip {
    display: flex;
    align-items: stretch;
    width: 100%;
    height: 2.2rem;
    --notch-color: var(--color-border);
    background: var(--corner-notch), rgba(255, 255, 255, 0.03);
    border: 1px solid var(--color-border-soft);
    cursor: pointer;
    font-family: var(--title-font-family);
    text-align: left;
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
    flex-shrink: 0;
    padding: 0 0.6rem;
    background: rgba(255, 255, 255, 0.05);
    border-right: 1px solid var(--color-border-soft);
    color: var(--color-text-light);
    font-size: 0.8rem;
    letter-spacing: 0.08em;
}

/* long project-ids are cut, the full id is shown in the title of the button */
.project-chip-value {
    flex: 1;
    min-width: 0;
    align-self: center;
    padding: 0 1rem 0 0.7rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text);
    font-size: 0.95rem;
    font-weight: 500;
    letter-spacing: 0.04em;
}

.sidebar-footer {
    display: flex;
    justify-content: space-between;
    padding: 0.8rem 1.6rem;
    border-top: 1px solid var(--color-border-soft);
    font-family: var(--title-font-family);
    font-size: 0.8rem;
    letter-spacing: 0.06em;
    color: var(--color-text-light);
}

.sidebar-version {
    font-variant-numeric: tabular-nums;
}
</style>
