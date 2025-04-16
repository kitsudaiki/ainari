/**
 * @file        structs.h
 *
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
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

#ifndef HANAMI_INTERNAL_STRUCTS_H
#define HANAMI_INTERNAL_STRUCTS_H

#include <src/common/defines.h>
#include <src/common/enums.h>
#include <stdint.h>

#include <cstring>

class Cluster;

namespace Hanami
{

struct WorkerTask {
    Cluster* cluster = nullptr;
    uint32_t hexagonId = UNINIT_STATE_32;
    uint32_t blockId = UNINIT_STATE_32;
    ClusterProcessingMode mode = ClusterProcessingMode::TRAIN_BACKWARD_MODE;
    uint8_t padding[7];
};
static_assert(sizeof(WorkerTask) == 24);

//==================================================================================================

}  // namespace Hanami

#endif  // HANAMI_INTERNAL_STRUCTS_H
