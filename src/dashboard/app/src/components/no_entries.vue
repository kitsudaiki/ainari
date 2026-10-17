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
    <!-- The message is placed into a table with the structure of the overview-table, so it is on
    the same height as the first row of a table with entries, also below the buttons of the card -->
    <table class="overview-table no-entries">
        <thead>
            <tr>
                <th>&nbsp;</th>
            </tr>
        </thead>
        <tbody>
            <tr>
                <td>{{ text }}</td>
            </tr>
        </tbody>
    </table>
</template>

<script setup lang="ts">
defineProps<{
    /** Message, which tells that there are no entries */
    text: string;
}>();
</script>

<style scoped>
/* only the position of the row is taken over, not its look */
.overview-table.no-entries tbody tr,
.overview-table.no-entries tbody tr:hover {
    background: none;
}

/* empty slot: dashed frame with faint diagonal stripes */
.overview-table.no-entries tbody td,
.overview-table.no-entries tbody tr:hover td {
    background: repeating-linear-gradient(
        -45deg,
        rgba(255, 255, 255, 0.025) 0 2px,
        transparent 2px 8px
    );
    border: 1px dashed var(--color-border-soft);
    box-shadow: inset 2px 0 0 var(--color-border-soft);
    color: var(--color-text-light);
    font-family: var(--title-font-family);
    font-size: 0.85rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    /* the only cell is the last one as well, which is aligned to the right for the actions */
    text-align: left;
    width: auto;
}
</style>
