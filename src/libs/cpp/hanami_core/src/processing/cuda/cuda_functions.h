/**
 * @file        cuda_functions.h
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

#ifndef CUDA_FUNCTIONS_H
#define CUDA_FUNCTIONS_H

#include <cluster/objects.h>

extern "C" Block* initDevice_CUDA(Block* hostBlocks, const uint32_t numberOfBlocks);

extern "C" void initHexagonOnDevice_CUDA(Hexagon* hexagon,
                                         ClusterSettings* clusterSettings,
                                         Block* hostBlocks,
                                         Block* deviceBlocks);

extern "C" void removeFromDevice_CUDA(Hexagon* hexagon);

extern "C" void copyFromGpu_CUDA(Hexagon* hexagon, Block* blocks, Block* deviceBlocks);

extern "C" void update_CUDA(Hexagon* hexagon, Block* deviceBlocks);

extern "C" void processing_CUDA(Hexagon* hexagon, Block* blocks, const bool doTrain);

extern "C" void backpropagation_CUDA(Hexagon* hexagon, Block* blocks);

extern "C" void reduction_CUDA(Hexagon* hexagon);

#endif  // CUDA_FUNCTIONS_H
