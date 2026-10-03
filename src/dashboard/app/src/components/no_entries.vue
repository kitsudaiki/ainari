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

.overview-table.no-entries tbody td,
.overview-table.no-entries tbody tr:hover td {
    background: none;
    border: none;
    box-shadow: none;
    color: var(--color-text);
    font-family: inherit;
    font-size: 1rem;
    font-weight: normal;
    /* the only cell is the last one as well, which is aligned to the right for the actions */
    text-align: left;
    width: auto;
}
</style>
