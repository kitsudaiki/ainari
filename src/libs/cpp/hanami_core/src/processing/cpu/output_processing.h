/**
 * @file        output_processing.h
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

#ifndef OUTPUTPROCESSING_H
#define OUTPUTPROCESSING_H

#include <cluster/cluster.h>
#include <cluster/objects.h>
#include <hanami_common/functions/common_functions.h>
#include <include/hanami_core/hanami_root.h>
#include <math.h>
#include <processing/cluster_resize.h>

#include <cmath>

/**
 * @brief _initConnection
 * @param out
 * @param connection
 */
inline void
_initConnection(OutputNeuron* out, float& connection)
{
    if (connection == 0.0f && out->exprectedVal > 0.0f) {
        constexpr float randMax = static_cast<float>(RAND_MAX);
        constexpr float sigNeg = 0.5f;

        connection = (static_cast<float>(rand()) / randMax) / 10.0f;
    }
}

/**
 * @brief process output-nodes
 *
 * @param hexagon current hexagon
 * @param randomSeed current seed for random-generation
 */
template <bool doTrain>
inline void
processNeuronsOfOutputHexagon(Hexagon* hexagon, uint32_t randomSeed)
{
    Axon* axon = nullptr;
    AxonBlock* axonBlock = nullptr;
    float weightSum = 0.0f;
    uint64_t outPos = 0;
    uint64_t wb = 0;
    uint32_t w = 0;

    OutputNeuron* out = nullptr;
    OutputWeightBlock* weightBlockSection = nullptr;
    OutputWeightBlock* wBlock = nullptr;
    OutputInterface* outputInterface = hexagon->outputInterface;

    if (outputInterface->weightBlocks.size() == 0) {
        return;
    }

    assert(outputInterface->weightBlocks.size() % outputInterface->outputNeurons.size() == 0);
    const uint64_t dim
        = outputInterface->weightBlocks.size() / outputInterface->outputNeurons.size();
    assert(dim == outputInterface->targetAxonBlocks.size());

    for (outPos = 0; outPos < outputInterface->outputNeurons.size(); ++outPos) {
        out = &outputInterface->outputNeurons[outPos];
        weightBlockSection = &outputInterface->weightBlocks[outPos * dim];
        weightSum = 0.0f;

        for (wb = 0; wb < dim; ++wb) {
            axonBlock = &outputInterface->targetAxonBlocks[wb];
            wBlock = &weightBlockSection[wb];

            for (w = 0; w < NEURONS_PER_BLOCK; ++w) {
                axon = &axonBlock->axons[w];
                axon->delta = 0.0f;

                if (axon->potential != 0.5f) {
                    if constexpr (doTrain) {
                        _initConnection(out, wBlock->connectionWeight[w]);
                    }
                    weightSum += axon->potential * wBlock->connectionWeight[w];
                }
            }
        }

        out->outputVal = 0.0f;
        if (weightSum != 0.0f) {
            out->outputVal = 1.0f / (1.0f + exp(-1.0f * weightSum));
        }
        // std::cout << out->outputVal << " : " << out->exprectedVal << std::endl;
    }
    // std::cout << "-------------------------------------" << std::endl;
}

#endif  // OUTPUTPROCESSING_H
