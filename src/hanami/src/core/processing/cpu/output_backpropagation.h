#ifndef OUTPUTBACKPROPAGATION_H
#define OUTPUTBACKPROPAGATION_H

#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/cpu/cpu_host.h>
#include <core/processing/logical_host.h>
#include <hanami_root.h>

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
    Axon* axon = nullptr;
    OutputNeuron* out = nullptr;
    OutputTargetLocationPtr* target = nullptr;
    constexpr float learnValue = 0.05f;
    float delta = 0.0f;
    float update = 0.0f;
    uint64_t i = 0;
    uint64_t j = 0;

    assert(outputInterface != nullptr);

    for (i = 0; i < outputInterface->outputNeurons.size(); ++i) {
        out = &outputInterface->outputNeurons[i];
        delta = out->outputVal - out->exprectedVal;
        update = delta * out->outputVal * (1 - out->outputVal);

        for (j = 0; j < NUMBER_OF_OUTPUT_CONNECTIONS; ++j) {
            target = &out->targets[j];

            if (target->blockId == UNINIT_STATE_32) {
                continue;
            }

            axon = &outputInterface->targetAxonBlocks[target->blockId].axons[target->neuronId];
            axon->delta += update * target->connectionWeight;
            target->connectionWeight -= update * learnValue * axon->potential;
        }
    }

    return true;
}

#endif  // OUTPUTBACKPROPAGATION_H
