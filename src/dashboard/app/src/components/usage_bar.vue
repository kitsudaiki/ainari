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
        <div
            class="usage-bar-fill"
            :class="level"
            :style="{ width: percentage + '%' }"
        ></div>
        <span class="usage-bar-text">{{ label }}</span>
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
.usage-bar {
    position: relative;
    width: 100%;
    min-width: 8rem;
    height: 1.25rem;
    border-radius: 0.25rem;
    overflow: hidden;
    background-color: var(--color-higlight-field);
}

.usage-bar-fill {
    height: 100%;
    transition:
        width 0.3s ease,
        background-color 0.3s ease;
}

.usage-bar-fill.ok {
    background-color: var(--color-usage-ok);
}

.usage-bar-fill.warning {
    background-color: var(--color-usage-warning);
}

.usage-bar-fill.critical {
    background-color: var(--color-usage-critical);
}

/* the shadow keeps the text readable on the empty and on the filled part of the bar */
.usage-bar-text {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    font-weight: bold;
    white-space: nowrap;
    color: var(--color-text);
    text-shadow: 0 0 0.2rem #000000;
}
</style>
