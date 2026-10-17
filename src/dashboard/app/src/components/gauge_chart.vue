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
    <div class="gauge-chart" :class="{ critical: percentage > 90 }">
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
                :stroke-width="stroke"
                :stroke-dasharray="dashArray"
                :stroke-dashoffset="progressOffset"
                :transform="rotation"
            />

            <!-- Center label: the value and below it the maximum -->
            <text
                class="gauge-label"
                :x="center"
                :y="center - 4"
                text-anchor="middle"
                dominant-baseline="middle"
            >
                {{ props.value }}
            </text>
            <text
                class="gauge-sublabel"
                :x="center"
                :y="center + 28"
                text-anchor="middle"
                dominant-baseline="middle"
            >
                / {{ props.max }}
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
    size: 190,
    stroke: 6,
});

const center = props.size / 2;

const radius = (props.size - props.stroke) / 2 - 1;

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
    --gauge-color: var(--color-highlight);
    --gauge-glow: var(--color-highlight-glow);

    background: transparent;
    padding-top: 1.4rem;

    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    font-family: var(--title-font-family);
    color: var(--color-text);
}

/* a nearly exhausted quota is shown in red */
.gauge-chart.critical {
    --gauge-color: var(--color-usage-critical);
    --gauge-glow: rgb(from var(--color-usage-critical) r g b / 0.45);
}

.gauge-bg {
    stroke: rgba(255, 255, 255, 0.07);
}

.gauge-progress {
    stroke: var(--gauge-color);
    filter: drop-shadow(0 0 0.35rem var(--gauge-glow));
    transition: stroke-dashoffset 1s ease-in-out; /* fill-animation of 1 second lenght*/
}

.gauge-label {
    fill: var(--color-text);
    font-family: var(--title-font-family);
    font-size: 3rem;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
}

.gauge-sublabel {
    fill: var(--color-text-light);
    font-family: var(--title-font-family);
    font-size: 1rem;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.05em;
}
</style>
