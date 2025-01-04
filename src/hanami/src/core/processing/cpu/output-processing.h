#ifndef OUTPUTPROCESSING_H
#define OUTPUTPROCESSING_H

#include <api/websocket/cluster_io.h>
#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/cluster_resize.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>
#include <math.h>

#include <cmath>

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
    // std::cout<<"processNeuronsOfOutputHexagon: h"<<hexagon->header.hexagonId<<std::endl;

    Axon* axon = nullptr;
    OutputTargetLocationPtr* target = nullptr;
    float weightSum = 0.0f;
    bool found = false;
    OutputInterface* outputInterface = hexagon->outputInterface;

    for (OutputNeuron& out : hexagon->outputInterface->outputNeurons) {
        weightSum = 0.0f;
        found = false;

        for (uint8_t j = 0; j < NUMBER_OF_OUTPUT_CONNECTIONS; ++j) {
            target = &out.targets[j];

            if constexpr (doTrain) {
                if (found == false && target->blockId == UNINIT_STATE_32 && out.exprectedVal > 0.0
                    && Hanami::pcg_hash(randomSeed) % 50 == 0)
                {
                    const uint32_t blockId = Hanami::pcg_hash(randomSeed)
                                             % hexagon->outputInterface->targetAxonBlocks.size();
                    const uint16_t neuronId
                        = Hanami::pcg_hash(randomSeed) % NEURONS_PER_NEURONBLOCK;
                    const float potential
                        = outputInterface->targetAxonBlocks[blockId].axons[neuronId].potential;

                    if (potential != 0.5f) {
                        target->blockId = blockId;
                        target->neuronId = neuronId;
                        target->connectionWeight
                            = ((float)Hanami::pcg_hash(randomSeed) / (float)RAND_MAX);
                        found = true;

                        if (potential < 0.5f) {
                            target->connectionWeight *= -1.0f;
                        }
                    }
                }
            }

            if (target->blockId == UNINIT_STATE_32) {
                continue;
            }

            axon = &outputInterface->targetAxonBlocks[target->blockId].axons[target->neuronId];
            weightSum += axon->potential * target->connectionWeight;
        }

        out.outputVal = 0.0f;
        if (weightSum != 0.0f) {
            out.outputVal = 1.0f / (1.0f + exp(-1.0f * weightSum));
        }
        // std::cout << out.outputVal << " : " << out.exprectedVal << std::endl;
    }
    // std::cout << "-------------------------------------" << std::endl;
}

#endif  // OUTPUTPROCESSING_H
