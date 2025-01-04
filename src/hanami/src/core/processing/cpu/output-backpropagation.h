#ifndef OUTPUTBACKPROPAGATION_H
#define OUTPUTBACKPROPAGATION_H

#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/cpu/cpu_host.h>
#include <core/processing/logical_host.h>
#include <hanami_root.h>
#include <math.h>

#include <cmath>

// Derivative of the activation function
inline float
_sigmoidDerivative(const float x)
{
    return x * (1 - x);
}

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
    // std::cout << "backpropagateOutput" << std::endl;

    Axon* axon = nullptr;
    OutputNeuron* out = nullptr;
    OutputTargetLocationPtr* target = nullptr;
    constexpr float learnValue = 0.05f;
    float totalDelta = 0.0f;
    float delta = 0.0f;
    float update = 0.0f;
    uint64_t i = 0;
    uint64_t j = 0;

    assert(outputInterface != nullptr);

    for (i = 0; i < outputInterface->targetAxonBlocks.size(); ++i) {
        for (j = 0; j < NEURONS_PER_NEURONBLOCK; ++j) {
            axon = &outputInterface->targetAxonBlocks[i].axons[j];
            axon->delta = 0.0f;
        }
    }

    for (i = 0; i < outputInterface->outputNeurons.size(); ++i) {
        out = &outputInterface->outputNeurons[i];
        delta = out->outputVal - out->exprectedVal;
        update = delta * _sigmoidDerivative(out->outputVal);

        for (j = 0; j < NUMBER_OF_OUTPUT_CONNECTIONS; ++j) {
            target = &out->targets[j];

            if (target->blockId == UNINIT_STATE_32) {
                continue;
            }

            axon = &outputInterface->targetAxonBlocks[target->blockId].axons[target->neuronId];
            axon->delta += update * target->connectionWeight;
            target->connectionWeight -= update * learnValue * axon->potential;

            totalDelta += abs(delta);
        }
    }

    // // std::cout << "totalDelta: " << totalDelta << std::endl;

    return true;
}

#endif  // OUTPUTBACKPROPAGATION_H
