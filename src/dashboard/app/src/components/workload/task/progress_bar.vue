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
    <div class="progress-bar-container">
        <div class="progress-bar" :style="{ width: progress + '%' }"></div>
        <span class="progress-text">{{ progress }}%</span>
    </div>
</template>

<script lang="ts">
import { defineComponent, ref, onMounted, onUnmounted } from "vue";
import axios, { AxiosInstance } from "axios";
import context from "../../../auth_context";

export default defineComponent({
    name: "ProgressBar",
    props: {
        task_uuid: {
            type: String,
            required: true,
        },
        cluster_uuid: {
            type: String,
            required: true,
        },
    },
    setup(props) {
        const progress = ref(0);
        let intervalId: number | undefined;

        // initialized once
        let hanami_api: AxiosInstance;
        let sakura_api: AxiosInstance;
        let authContext: any;

        const initApis = async () => {
            authContext = context.getAuthContext();

            hanami_api = axios.create({
                baseURL: authContext.hanami_address,
                headers: { Authorization: `Bearer ${authContext.token}` },
            });

            // get torii port only once
            const cluster_response = await hanami_api.get(
                `/v1alpha/cluster/${props.cluster_uuid}`,
            );

            const torii_port = cluster_response.data.torii_port;

            sakura_api = axios.create({
                baseURL: `${authContext.torii_base_address}:${torii_port}`,
                headers: { Authorization: `Bearer ${authContext.token}` },
            });
        };

        const fetchProgress = async () => {
            try {
                const task_response = await sakura_api.get(
                    `/v1alpha/cluster/${props.cluster_uuid}/task/${props.task_uuid}`,
                );

                // calculate current progress
                let state =
                    (100 / task_response.data.total_number_of_cycles) *
                    task_response.data.total_number_of_epochs;
                state *=
                    task_response.data.current_epoch *
                        task_response.data.total_number_of_cycles +
                    task_response.data.current_cycle;

                // workaround for the finish-state, when the epoch becomes 1 too big for the calculation
                if (state > 100) state = 100;

                // round value to full integer
                progress.value = Math.ceil(state);

                if (progress.value >= 100 && intervalId) {
                    // if progress is finished, than stop the update 
                    // to avoid unnecessary requests to the backend
                    clearInterval(intervalId);
                }
            } catch (err) {
                console.error("Failed to load tasks", err);
            }
        };

        onMounted(async () => {
            await initApis();
            await fetchProgress();
            intervalId = window.setInterval(fetchProgress, 1000);
        });

        onUnmounted(() => {
            if (intervalId) clearInterval(intervalId);
        });

        return { progress };
    },
});
</script>

<style scoped>
.progress-bar-container {
    width: 100%;
    background-color: var(--color-text);
    position: relative;
    height: 20px;
}

.progress-bar {
    height: 100%;
    background-color: var(--color-highlight);
    transition: width 0.3s ease;
}

.progress-text {
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    font-size: 12px;
    font-weight: bold;
    color: black; /*  invert(var(--color-text)) doesn't for unknown reason*/
}
</style>
