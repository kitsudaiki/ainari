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
    <div class="modal-overlay" @click.self="cancel">
        <div class="modal checkpoint-delete-modal">
            <div class="modal-topbar">
                <span>Delete checkpoint</span>
            </div>
            <div class="modal-content">
                <p>Are you sure you want to delete?</p>
                <strong>Checkpoint: {{ checkpoint?.uuid }}</strong>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        @click="handleAccept(checkpoint?.uuid)"
                    >
                        <img :src="icons.acceptIcon" alt="Accept" />
                    </button>
                    <button class="icon-button" @click="cancel">
                        <img :src="icons.cancelIcon" alt="Cancel" />
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import api from "../../../api";

interface Props {
    checkpoint: { uuid: string; name: string } | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

async function handleAccept(checkpoint_uuid: string) {
    if (!checkpoint_uuid) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.hanami_api.delete(`/v1alpha/checkpoint/${checkpoint_uuid}`, {
            headers: { Authorization: `Bearer ${token}` },
        });
    } catch (err) {
        console.error("Failed to delete checkpoint", err);
    }

    emit("accept");
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.checkpoint-delete-modal {
    height: 16rem;
    width: 20rem;
}
</style>
