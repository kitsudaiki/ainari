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
    <div class="card">
        <div class="card-label">Network filter{{ titleSuffix }}</div>
        <div class="card-content">
            <!-- Back button -->
            <button class="back-button" @click="switchToVirtualMachines">
                ←
            </button>
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="rules.length > 0">
                <thead>
                    <tr>
                        <th>Direction</th>
                        <th>Type</th>
                        <th>Rule</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="rule in rules" :key="ruleKey(rule)">
                        <td>{{ directionLabels[rule.direction] }}</td>
                        <td>{{ typeLabels[rule.type] }}</td>
                        <td>{{ rule.spec }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(ruleKey(rule))"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === ruleKey(rule)"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openRemoveModal(rule)">
                                        Remove
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <NoEntries
                v-else
                text="No filter rules found, all traffic is allowed"
            />
        </div>

        <NetworkFilterAddModal
            v-if="showAddModal"
            :virtual_machine_uuid="props.id"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <NetworkFilterRemoveModal
            v-if="showRemoveModal"
            :virtual_machine_uuid="props.id"
            :rule="ruleToRemove"
            :icons="icons"
            @accept="acceptRemoveModal"
            @cancel="cancelRemoveModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, inject } from "vue";

import { hanami } from "@/api";
import type { FilterDirection } from "@/api";
import NetworkFilterAddModal from "./network_filter_add_modal.vue";
import NetworkFilterRemoveModal from "./network_filter_remove_modal.vue";
import type { FilterRule, FilterRuleType } from "./network_filter_rule";
import { handleAxiosError } from "@/handleAxiosError";
import NoEntries from "@/components/no_entries.vue";

const props = defineProps<{
    id: string | null;
}>();

const errorPopupMsg = ref<string>("");
const rules = ref<FilterRule[]>([]);
const virtualMachineName = ref<string>("");
const openDropdown = ref<string | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;
const showAddModal = ref(false);
const showRemoveModal = ref(false);
const ruleToRemove = ref<FilterRule | null>(null);

const directionLabels: Record<FilterDirection, string> = {
    ingress: "Ingress",
    egress: "Egress",
};
const typeLabels: Record<FilterRuleType, string> = {
    ip_range: "IP range",
    port: "Port",
};

const titleSuffix = computed(() =>
    virtualMachineName.value ? ` of ${virtualMachineName.value}` : "",
);

const emit = defineEmits<{
    (e: "change-view", payload: { view: string; id: string | null }): void;
}>();

function ruleKey(rule: FilterRule): string {
    return `${rule.direction}/${rule.type}/${rule.spec}`;
}

/**
 * The filters of both directions are read from the list of hanami, which
 * contains only the directions, which have a filter. Each entry of their
 * include-lists becomes one rule of the table.
 */
async function fetchRules() {
    if (!props.id) return;

    try {
        const filters = await hanami.listNetworkFilters();
        const result: FilterRule[] = [];
        // ingress first, so the order doesn't change between the reloads
        for (const direction of ["ingress", "egress"] as FilterDirection[]) {
            const filter = filters.find(
                (entry) =>
                    entry.virtual_machine_uuid === props.id &&
                    entry.direction === direction,
            );
            if (!filter) continue;
            for (const spec of filter.ip_ranges) {
                result.push({ direction, type: "ip_range", spec });
            }
            for (const spec of filter.ports) {
                result.push({ direction, type: "port", spec });
            }
        }
        rules.value = result;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load network filter",
        );
    }
}

async function fetchVirtualMachineName() {
    if (!props.id) return;
    try {
        const virtualMachine = await hanami.getVirtualMachine(props.id);
        virtualMachineName.value = virtualMachine.name;
    } catch {
        // the name is only shown in the title, so the uuid-based view still works
    }
}

//=============================================================================
// Switch back to the list of virtual machines
//=============================================================================
function switchToVirtualMachines() {
    emit("change-view", { view: "WorkloadVirtualMachine", id: null });
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(key: string) {
    openDropdown.value = openDropdown.value === key ? null : key;
}

function handleClickOutside(event: MouseEvent) {
    const dropdowns = document.querySelectorAll(".table-dropdown");
    let clickedInside = false;
    dropdowns.forEach((dropdown) => {
        if (dropdown.contains(event.target as Node)) {
            clickedInside = true;
        }
    });
    if (!clickedInside) {
        openDropdown.value = null; // close the dropdown
    }
}

//=============================================================================
// Add modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}
async function acceptAddModal() {
    await fetchRules();
    cancelAddModal();
}

//=============================================================================
// Remove modal
//=============================================================================
function openRemoveModal(rule: FilterRule) {
    ruleToRemove.value = rule;
    showRemoveModal.value = true;
    openDropdown.value = null;
}
function cancelRemoveModal() {
    showRemoveModal.value = false;
    ruleToRemove.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptRemoveModal() {
    await fetchRules();
    cancelRemoveModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchRules);
onMounted(fetchVirtualMachineName);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.overview-table td:nth-child(1),
.overview-table td:nth-child(2) {
    width: 12rem;
}

/* the first column contains the direction and not a UUID like in the other
overviews, so it doesn't need the mono-space font of the UUIDs */
.overview-table tbody td:nth-child(1) {
    font-family: inherit;
    font-size: inherit;
    color: inherit;
}
</style>
