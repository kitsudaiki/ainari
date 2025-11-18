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
        <div class="modal cluster-delete-modal">
            <div class="modal-topbar">
                <span>Delete cluster</span>
            </div>
            <div class="modal-content">
                <p>Are you sure you want to delete?</p>
                <strong>Cluster: {{ cluster?.uuid }}</strong>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        @click="handleAccept(cluster?.uuid)"
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
    cluster: { id: number; name: string } | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

async function handleAccept(cluster_uuid: string) {
    if (!cluster_uuid) return;
    try {
        const token = localStorage.getItem("jwtToken");
        await api.sakura_api.delete(`/v1alpha/cluster/${cluster_uuid}`, {
            headers: { Authorization: `Bearer ${token}` },
        });
    } catch (err) {
        console.error("Failed to delete cluster", err);
    }

    emit("accept");
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.cluster-delete-modal {
    height: 16rem;
    width: 20rem;
}
</style>
