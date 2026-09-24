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
    <div class="card">
        <div class="card-label">Images</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openAddModal">+</button>

            <table class="overview-table" v-if="images.length > 0">
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th>Rows</th>
                        <th>Columns</th>
                        <th>Snapshot</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr v-for="image in images" :key="image.uuid">
                        <td>{{ image.uuid }}</td>
                        <td>{{ image.name }}</td>
                        <td>{{ image.number_of_rows }}</td>
                        <td>{{ image.number_of_columns }}</td>
                        <td>{{ image.is_snapshot ? "Yes" : "No" }}</td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="toggleDropdown(image.uuid)"
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === image.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button @click="openInfoModal(image)">
                                        Info
                                    </button>
                                    <button @click="openDeleteModal(image)">
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No images found</p>
        </div>

        <ImageCreateModal
            v-if="showAddModal"
            :icons="icons"
            @accept="acceptAddModal"
            @cancel="cancelAddModal"
        />

        <ImageInfoModal
            v-if="showInfoModal"
            :image="imageToShow"
            :icons="icons"
            @cancel="cancelInfoModal"
        />

        <ImageDeleteModal
            v-if="showDeleteModal"
            :image="imageToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";

import { ryokan } from "@/api";
import type { ImageBasicResp } from "@/api";
import ImageCreateModal from "./image_create_modal.vue";
import ImageInfoModal from "./image_info_modal.vue";
import ImageDeleteModal from "./image_delete_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const images = ref<ImageBasicResp[]>([]);
const showAddModal = ref(false);
const showInfoModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const imageToShow = ref<ImageBasicResp | null>(null);
const imageToDelete = ref<ImageBasicResp | null>(null);
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

async function fetchImages() {
    try {
        images.value = await ryokan.listImages();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load images");
    }
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(uuid: string) {
    openDropdown.value = openDropdown.value === uuid ? null : uuid;
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
// Add image modal
//=============================================================================
function openAddModal() {
    showAddModal.value = true;
}
function cancelAddModal() {
    showAddModal.value = false;
}
async function acceptAddModal() {
    await fetchImages();
    cancelAddModal();
}

//=============================================================================
// Info modal
//=============================================================================
function openInfoModal(image: ImageBasicResp) {
    imageToShow.value = image;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    imageToShow.value = null;
    openDropdown.value = null;
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(image: ImageBasicResp) {
    imageToDelete.value = image;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    imageToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchImages();
    cancelDeleteModal();
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchImages);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
});
</script>

<style scoped>
.overview-table td:nth-child(3),
.overview-table td:nth-child(4) {
    width: 8rem;
}
</style>
