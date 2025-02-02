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
    constexpr float learnValue = 0.05f;
    float delta = 0.0f;
    float update = 0.0f;
    uint32_t outPos = 0;
    uint32_t wb = 0;
    uint32_t w = 0;
    assert(outputInterface != nullptr);

    Axon* axon = nullptr;
    AxonBlock* axonBlock = nullptr;
    OutputNeuron* out = nullptr;
    OutputWeightBlock* weightBlocks = nullptr;
    OutputWeightBlock* wBlock = nullptr;

    if (outputInterface->weights.size() == 0) {
        return true;
    }

    assert(outputInterface->weights.size() % outputInterface->outputNeurons.size() == 0);
    const uint32_t dim = outputInterface->weights.size() / outputInterface->outputNeurons.size();

    for (outPos = 0; outPos < outputInterface->outputNeurons.size(); ++outPos) {
        out = &outputInterface->outputNeurons[outPos];
        weightBlocks = &outputInterface->weights[outPos * dim];

        delta = out->outputVal - out->exprectedVal;
        update = delta * out->outputVal * (1 - out->outputVal);

        for (wb = 0; wb < outputInterface->weights.size();
             wb += outputInterface->outputNeurons.size())
        {
            wBlock = &weightBlocks[wb];
            axonBlock = &outputInterface->targetAxonBlocks[wb];

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
