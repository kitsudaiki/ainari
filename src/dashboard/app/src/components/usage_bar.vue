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

<!--
Small bar, which shows how much of a resource is used. The color of the bar changes with the
usage: green up to 75%, yellow up to 90% and red above.
-->
<template>
    <div
        class="usage-bar"
        role="progressbar"
        :aria-valuenow="percentage"
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-label="label"
        :title="`${label} (${percentage}%)`"
    >
        <div class="usage-bar-text">
            <span>{{ label }}</span>
            <span class="usage-bar-percentage" :class="level"
                >{{ percentage }}%</span
            >
        </div>
        <div class="usage-bar-track">
            <div
                class="usage-bar-fill"
                :class="level"
                :style="{ width: percentage + '%' }"
            ></div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

interface Props {
    /** amount of the resource, which is already in use */
    used: number;
    /** total amount of the resource */
    total: number;
    /** unit, which is appended to the values in the text of the bar */
    unit?: string;
}

const props = withDefaults(defineProps<Props>(), {
    unit: "",
});

// a host without the resource at all is shown as empty instead of dividing by 0
const percentage = computed(() => {
    if (props.total <= 0) return 0;
    const value = (props.used / props.total) * 100;
    return Math.min(100, Math.max(0, Math.round(value)));
});

const level = computed(() => {
    if (percentage.value > 90) return "critical";
    if (percentage.value >= 75) return "warning";
    return "ok";
});

const label = computed(() => {
    const unit = props.unit ? ` ${props.unit}` : "";
    return `${props.used} / ${props.total}${unit}`;
});
</script>

<style scoped>
/* slim meter with the values above it, so the text never lies on top of the bar */
.usage-bar {
    --usage-color: var(--color-usage-ok);

    width: 100%;
    min-width: 8rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}

.usage-bar-text {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    font-family: var(--title-font-family);
    font-size: 0.8rem;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    white-space: nowrap;
    color: var(--color-text-light);
}

/* the track is split into 20 segments by the mask, the fill is cut by the same mask */
.usage-bar-track {
    height: 0.45rem;
    background: rgba(255, 255, 255, 0.08);
    mask-image: repeating-linear-gradient(
        90deg,
        #000 0 calc(5% - 2px),
        transparent calc(5% - 2px) 5%
    );
}

.usage-bar-fill {
    height: 100%;
    background-color: var(--usage-color);
    transition: width 0.3s ease;
}

.usage-bar-fill.warning,
.usage-bar-percentage.warning {
    --usage-color: var(--color-usage-warning);
}

.usage-bar-fill.critical,
.usage-bar-percentage.critical {
    --usage-color: var(--color-usage-critical);
}

.usage-bar-percentage {
    color: var(--usage-color);
}
</style>
