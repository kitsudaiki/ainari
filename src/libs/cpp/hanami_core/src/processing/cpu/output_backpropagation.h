/**
 * @file        output_backpropagation.h
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

#ifndef OUTPUTBACKPROPAGATION_H
#define OUTPUTBACKPROPAGATION_H

#include <cluster/cluster.h>
#include <cluster/objects.h>
#include <include/hanami_core/hanami_root.h>
#include <processing/cpu/cpu_host.h>
#include <processing/logical_host.h>

/**
 * @brief backpropagate output-nodes
 *
 * @param hexagon pointer to current hexagon
 *
 * @return always true
 */
inline bool
backpropagateOutput(OutputInterface* outputInterface)
{
    constexpr float learnValue = 0.05f;
    float delta = 0.0f;
    float update = 0.0f;
    uint64_t outPos = 0;
    uint64_t wb = 0;
    uint32_t w = 0;
    assert(outputInterface != nullptr);

    Axon* axon = nullptr;
    AxonBlock* axonBlock = nullptr;
    OutputNeuron* out = nullptr;
    OutputWeightBlock* weightBlockSection = nullptr;
    OutputWeightBlock* wBlock = nullptr;

    if (outputInterface->weightBlocks.size() == 0) {
        return true;
    }

    assert(outputInterface->weightBlocks.size() % outputInterface->outputNeurons.size() == 0);
    const uint64_t dim
        = outputInterface->weightBlocks.size() / outputInterface->outputNeurons.size();
    assert(dim == outputInterface->targetAxonBlocks.size());

    for (outPos = 0; outPos < outputInterface->outputNeurons.size(); ++outPos) {
        out = &outputInterface->outputNeurons[outPos];
        weightBlockSection = &outputInterface->weightBlocks[outPos * dim];

        delta = out->outputVal - out->exprectedVal;
        update = delta * out->outputVal * (1 - out->outputVal);

        for (wb = 0; wb < dim; ++wb) {
            axonBlock = &outputInterface->targetAxonBlocks[wb];
            wBlock = &weightBlockSection[wb];

            for (w = 0; w < NEURONS_PER_BLOCK; ++w) {
                axon = &axonBlock->axons[w];
                if (axon->potential != 0.5f) {
                    axon->delta += update * wBlock->connectionWeight[w];
                    wBlock->connectionWeight[w] -= update * learnValue * axon->potential;
                }
            }
        }
    }

    return true;
}

#endif  // OUTPUTBACKPROPAGATION_H
