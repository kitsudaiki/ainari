/**
 * @file       memory.cpp
 *
 *  @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *  @copyright  Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker
 *
 *      Licensed under the Apache License, Version 2.0 (the "License");
 *      you may not use this file except in compliance with the License.
 *      You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 *      Unless required by applicable law or agreed to in writing, software
 *      distributed under the License is distributed on an "AS IS" BASIS,
 *      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *      See the License for the specific language governing permissions and
 *      limitations under the License.
 */

#include <hanami_cpu/memory.h>

namespace Hanami
{

/**
 * @brief get total amount of main-memory of the system in bytes
 */
uint64_t
getTotalMemory()
{
    struct sysinfo info;
    if (sysinfo(&info) != 0) {
        return 0;
    }

    return static_cast<uint64_t>(info.totalram) * info.mem_unit;
}

/**
 * @brief get amound of free main-memory of the system in bytes
 */
uint64_t
getFreeMemory()
{
    struct sysinfo info;
    if (sysinfo(&info) != 0) {
        return 0;
    }

    return static_cast<uint64_t>(info.freeram + info.bufferram) * info.mem_unit;
}

/**
 * @brief get page-size of the main-memory of the system in bytes
 */
uint64_t
getPageSize()
{
    struct sysinfo info;
    if (sysinfo(&info) != 0) {
        return 0;
    }

    return static_cast<uint64_t>(info.mem_unit);
}

}  // namespace Hanami
