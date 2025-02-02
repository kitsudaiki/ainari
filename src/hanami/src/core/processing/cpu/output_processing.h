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
    uint32_t outPos = 0;
    uint32_t wb = 0;
    uint32_t w = 0;

    OutputNeuron* out = nullptr;
    OutputWeightBlock* weightBlocks = nullptr;
    OutputWeightBlock* wBlock = nullptr;
    OutputInterface* outputInterface = hexagon->outputInterface;

    if (outputInterface->weights.size() == 0) {
        return;
    }

    assert(outputInterface->weights.size() % outputInterface->outputNeurons.size() == 0);
    const uint32_t dim = outputInterface->weights.size() / outputInterface->outputNeurons.size();

    for (outPos = 0; outPos < outputInterface->outputNeurons.size(); ++outPos) {
        out = &outputInterface->outputNeurons[outPos];
        weightBlocks = &outputInterface->weights[outPos * dim];
        weightSum = 0.0f;

        for (wb = 0; wb < outputInterface->weights.size();
             wb += outputInterface->outputNeurons.size())
        {
            axonBlock = &outputInterface->targetAxonBlocks[wb];
            wBlock = &weightBlocks[wb];

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
