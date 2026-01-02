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
    <div class="gauge-chart">
        <svg :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`">
            <!-- Background arc -->
            <circle
                class="gauge-bg"
                :cx="center"
                :cy="center"
                :r="radius"
                fill="none"
                :stroke-width="stroke"
                :stroke-dasharray="dashArray"
                :stroke-dashoffset="backgroundOffset"
                :transform="rotation"
            />

            <!-- Progress arc -->
            <circle
                class="gauge-progress"
                :cx="center"
                :cy="center"
                :r="radius"
                fill="none"
                :stroke-width="stroke + 2"
                :stroke-dasharray="dashArray"
                :stroke-dashoffset="progressOffset"
                :transform="rotation"
            />

            <!-- Center label -->
            <text
                class="gauge-label"
                :x="center"
                :y="center"
                text-anchor="middle"
                dominant-baseline="middle"
            >
                {{ props.value }} / {{ props.max }}
            </text>
        </svg>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

interface Props {
    value: number;
    max: number;
    size?: number;
    stroke?: number;
}

const props = withDefaults(defineProps<Props>(), {
    size: 200,
    stroke: 14,
});

const center = props.size / 2;

const maxStroke = props.stroke + 2;
const radius = (props.size - maxStroke) / 2;

const circumference = 2 * Math.PI * radius;

const gaugePortion = 0.75;
const dashArray = circumference;

const percentage = computed(() =>
    Math.min(100, Math.round((props.value / props.max) * 100)),
);

const rotation = `rotate(-225 ${center} ${center})`;

const backgroundOffset = circumference * (1 - gaugePortion);

const progressOffset = computed(() => {
    return circumference * (1 - gaugePortion * (percentage.value / 100));
});
</script>

<style scoped>
.gauge-chart {
    background: transparent;
    padding-top: 2rem;

    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    font-family: var(--text-font-family);
    color: var(--color-text);
}

.gauge-bg {
    stroke: rgba(0, 0, 0, 0.1);
}

.gauge-progress {
    stroke: var(--color-highlight);
}

.gauge-label {
    fill: var(--color-text);
    font-family: var(--text-font-family);
    font-size: 2rem;
    font-weight: bold;
}
</style>
