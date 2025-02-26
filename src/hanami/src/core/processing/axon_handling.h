/**
 * @file        axon_handling.h
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

#ifndef AXON_HANDLING_H
#define AXON_HANDLING_H

#include <api/websocket/cluster_io.h>
#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/cluster_resize.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>

/**
 * @brief send one axon-block to the next hexagon
 *
 * @param cluster cluster, where the hexagon belongs to
 * @param sourceAxonBlock axon-block, which should to transfered
 */
inline void
_transferAxonBlocks(Cluster* cluster, AxonBlock* sourceAxonBlock)
{
    if (sourceAxonBlock->targetBlockId == UNINIT_STATE_32
        || sourceAxonBlock->targetHexagonId == UNINIT_STATE_32)
    {
        return;
    }

    Hexagon* targetHexagon = &cluster->hexagons[sourceAxonBlock->targetHexagonId];
    targetHexagon->transferAxonBlocks[sourceAxonBlock->targetBlockId] = *sourceAxonBlock;
}

/**
 * @brief send axon-blocks a hexagon to its connected output-interface
 *
 * @param hexagon pointer to hexagon to process
 */
inline void
_transferAxonBlockToOutput(Hexagon* hexagon)
{
    // updated blocks of output-interface
    OutputInterface* outputInterface = hexagon->outputInterface;
    if (outputInterface->targetAxonBlocks.size() != hexagon->axonBlocks.size()) {
        outputInterface->targetAxonBlocks.resize(hexagon->axonBlocks.size());
        const int64_t totalWeightBlocks
            = hexagon->axonBlocks.size() * outputInterface->outputNeurons.size();
        const int64_t diff = totalWeightBlocks - outputInterface->weights.size();

        // update weight-blocks
        if (diff > 0) {
            outputInterface->weights.resize(totalWeightBlocks, OutputWeightBlock());
        }
    }

    for (uint64_t blockId = 0; blockId < hexagon->axonBlocks.size(); ++blockId) {
        AxonBlock* sourceAxonBlock = &hexagon->axonBlocks[blockId];
        outputInterface->targetAxonBlocks[blockId] = *sourceAxonBlock;
    }
}

/**
 * @brief send axon-blocks from an input-interface to its connected hexagon
 *
 * @param cluster cluster, where the hexagon belongs to
 * @param inputInterface pointer to input-interface, of which the axon-blocks
 *                       should be send to the connected hexagon
 */
template <bool doTrain>
inline void
transferInputAxonBlocks(Cluster* cluster, InputInterface* inputInterface)
{
    const uint64_t targetId = inputInterface->targetHexagonId;
    cluster->hexagons[targetId].transferAxonBlocks.resize(inputInterface->inputAxons.size());

    for (uint64_t blockId = 0; blockId < inputInterface->inputAxons.size(); ++blockId) {
        AxonBlock* axonBlock = &inputInterface->inputAxons[blockId];

        if constexpr (doTrain) {
            if (axonBlock->targetBlockId == UNINIT_STATE_32) {
                axonBlock->targetHexagonId = targetId;
                axonBlock->targetBlockId = blockId;
            }
        }

        _transferAxonBlocks(cluster, axonBlock);
    }
}

/**
 * @brief process axon-blocks by initializing them and send them to the
 *        target-hexagon
 *
 * @param cluster pointer to cluster, where the hexagon belongs to
 * @param hexagon pointer to hexagon to process
 * @param randomSeed reference to the current seed of the randomizer
 */
template <bool doTrain>
inline void
transferAxonBlocks(Cluster* cluster, Hexagon* hexagon, uint32_t& randomSeed)
{
    // handle output-interface
    if (hexagon->outputInterface != nullptr) {
        _transferAxonBlockToOutput(hexagon);
        return;
    }

    // handle normal connection
    for (uint32_t sourceBlockId = 0; sourceBlockId < hexagon->axonBlocks.size(); ++sourceBlockId) {
        AxonBlock* axon = &hexagon->axonBlocks[sourceBlockId];
        if constexpr (doTrain) {
            if (axon->targetBlockId == UNINIT_STATE_32) {
                // get and update target
                const uint64_t randPos = Hanami::pcg_hash(randomSeed) % NUMBER_OF_POSSIBLE_NEXT;
                const uint64_t targetId = hexagon->possibleHexagonTargetIds[randPos];
                const uint64_t currentSize = cluster->hexagons[targetId].transferAxonBlocks.size();
                cluster->hexagons[targetId].transferAxonBlocks.resize(currentSize + 1);

                // update information in the source axon-block
                axon->targetHexagonId = targetId;
                axon->targetBlockId = cluster->hexagons[targetId].transferAxonBlocks.size() - 1;
                axon->sourceBlockId = sourceBlockId;
                axon->sourceHexagonId = hexagon->header.hexagonId;
            }
        }

        _transferAxonBlocks(cluster, axon);
    }
}

/**
 * @brief send one transfer-axon-block back to their source
 *
 * @param cluster pointer to cluster
 * @param axonBlock block to transfer
 */
inline void
_transferAxonBlocksBackwards(Cluster* cluster, AxonBlock* axonBlock)
{
    if (axonBlock->sourceBlockId == UNINIT_STATE_32
        || axonBlock->sourceHexagonId == UNINIT_STATE_32)
    {
        return;
    }

    Hexagon* sourceHexagon = &cluster->hexagons[axonBlock->sourceHexagonId];
    sourceHexagon->axonBlocks[axonBlock->sourceBlockId] = *axonBlock;
}

/**
 * @brief send axon-blocks of an output-interface back to the hexagon
 *
 * @param hexagon pointer to current processed hexagon
 */
inline void
transferAxonBlockFromOutput(Hexagon* hexagon)
{
    // std::cout << "transferAxonBlockFromOutput: x" << hexagon->header.hexagonId << std::endl;

    for (uint64_t blockId = 0; blockId < hexagon->axonBlocks.size(); ++blockId) {
        AxonBlock* axonBlock = &hexagon->axonBlocks[blockId];
        *axonBlock = hexagon->outputInterface->targetAxonBlocks[blockId];
    }
}

/**
 * @brief fake-transfer to the input just to finish the cycle properly
 *
 * @param hexagon pointer to current processed hexagon
 */
inline void
transferAxonBlockToInput(Hexagon* hexagon)
{
    for (uint64_t blockId = 0; blockId < hexagon->transferAxonBlocks.size(); ++blockId) {
        AxonBlock* axonBlock = &hexagon->transferAxonBlocks[blockId];
        hexagon->inputInterface->inputAxons[blockId] = *axonBlock;
    }
}

/**
 * @brief send transfer-axon-blocks back to their source
 *
 * @param cluster pointer to cluster
 * @param hexagon pointer to current processed hexagon
 */
inline void
transferAxonBlocksBackward(Cluster* cluster, Hexagon* hexagon)
{
    for (uint64_t blockId = 0; blockId < hexagon->transferAxonBlocks.size(); ++blockId) {
        AxonBlock* transferAxonBlock = &hexagon->transferAxonBlocks[blockId];
        _transferAxonBlocksBackwards(cluster, transferAxonBlock);
    }
}

/**
 * @brief handleTargetAxonBlocks
 * @param hexagon
 */
inline void
processTransferAxonBlocks(Cluster* cluster, Hexagon* hexagon, uint32_t& randomSeed)
{
    Axon* axon = nullptr;
    AxonBlock* axonBlock = nullptr;
    Hanami::ItemBuffer<Block>* blockBuffer = &hexagon->attachedHost->blocks;
    for (uint32_t blockId = 0; blockId < hexagon->transferAxonBlocks.size(); ++blockId) {
        axonBlock = &hexagon->transferAxonBlocks[blockId];

        for (uint16_t axonId = 0; axonId < NEURONS_PER_BLOCK; ++axonId) {
            axon = &axonBlock->axons[axonId];

            if (axon->activeCounter > 0 || axon->potential < 0.00001f) {
                continue;
            }

            // get target objects
            const TargetLocation loc = searchTargetInHexagon(hexagon, *blockBuffer);
            if (loc.targetBlock == UNINIT_STATE_32 || loc.targetConnection == UNINIT_STATE_16) {
                return;
            }

            // initialize found entry
            uint32_t randomSeed = rand();
            Block* blocks = Hanami::getItemData<Block>(*blockBuffer);
            Connection* targetConnection
                = &blocks[loc.targetBlock].connections[loc.targetConnection];
            if (initConnection(hexagon, targetConnection, randomSeed) == false) {
                // TODO: better error-handling
                continue;
            }

            targetConnection->active = true;
            targetConnection->lowerBound = 0.0f;
            targetConnection->sourceBlockId = blockId;
            targetConnection->sourceId = axonId;

            hexagon->header.numberOfFreeSections--;
            hexagon->wasResized = true;
            cluster->metrics.numberOfSections++;

            // std::cout<<"create "<<hexagon->header.hexagonId<<std::endl;
            axon->activeCounter = 1;
        }
    }
}

#endif  // AXON_HANDLING_H
