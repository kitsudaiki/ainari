/**
 * @file        cluster.cpp
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

#include "cluster_link.h"

#include <src/cluster/cluster.h>
#include <src/cluster/cluster_io_convert.h>
#include <src/common/logger.h>
#include <src/io/checkpoint/disc/checkpoint_io.h>
#include <src/processing/logical_host.h>

#include <iostream>

ClusterLink::ClusterLink(Cluster* cluster) { m_cluster = cluster; }

ClusterLink::~ClusterLink()
{
    if (m_cluster != nullptr) {
        delete m_cluster;
    }
}

/**
 * @brief ClusterLink::printMetrics
 */
void
ClusterLink::printMetrics() const
{
    std::cout << "Metrics of cluster " << m_cluster->clusterHeader.uuid.toString() << ": "
              << std::endl;
    std::cout << "    Number of hexagons: " << m_cluster->hexagons.size() << std::endl;
}

/**
 * @brief ClusterLink::createCheckpoint
 * @param targetFilePath
 * @return
 */
int
ClusterLink::createCheckpoint(const std::string& targetFilePath)
{
    Hanami::ErrorContainer error;
    std::filesystem::path filePath = targetFilePath;

    // cluster->stateMachine
    for (Hexagon& hexagon : m_cluster->hexagons) {
        hexagon.attachedHost->syncWithHost(&hexagon);
    }
    CheckpointIO m_clusterIO;
    ReturnStatus ret = m_clusterIO.writeClusterToFile(*m_cluster, targetFilePath, error);
    if (ret != OK) {
        std::cout << "error: " << error.toString() << std::endl;
        return ret;
    }

    return OK;
}

/**
 * @brief ClusterLink::fillInput
 * @param hexagonName
 * @param input
 * @param numberOfInputs
 */
bool
ClusterLink::fillInput(const std::string& hexagonName,
                       const float* input,
                       const uint64_t numberOfInputs)
{
    auto it = m_cluster->inputInterfaces.find(hexagonName);
    if (it == m_cluster->inputInterfaces.end()) {
        return false;
    }

    InputInterface* inputInterface = &it->second;
    inputInterface->initBuffer(numberOfInputs, 1);

    AxonBlock* axonBlock = nullptr;
    Axon* axon = nullptr;
    uint64_t blockId = 0;
    uint16_t axonId = 0;
    uint64_t counter = 0;
    float val = 0.0f;

    for (uint64_t i = 0; i < numberOfInputs; ++i) {
        val = input[i];
        // std::cout << "input: " << val << std::endl;
        blockId = counter / NEURONS_PER_BLOCK;
        axonId = counter % NEURONS_PER_BLOCK;
        axonBlock = &inputInterface->inputAxons[blockId];
        axonBlock->axons[axonId].potential = 0.0f;
        axonBlock->axons[axonId + 1].potential = 0.0f;
        axon = &axonBlock->axons[axonId + (val >= 0.0f)];
        axon->potential = abs(val);
        counter += 2;
    }

    return true;
}

/**
 * @brief ClusterLink::fillExpected
 * @param hexagonName
 * @param output
 * @param numberOfOutputs
 */
bool
ClusterLink::fillExpected(const std::string& hexagonName,
                          float* output,
                          const uint64_t numberOfOutputs)
{
    auto it = m_cluster->outputInterfaces.find(hexagonName);
    if (it == m_cluster->outputInterfaces.end()) {
        return false;
    }

    // float val = 0.0f;
    // for (uint64_t i = 0; i < numberOfOutputs; ++i) {
    //     val = output[i];
    //     std::cout << "output: " << val << std::endl;
    // }
    OutputInterface* outputInterface = &it->second;
    outputInterface->initBuffer(numberOfOutputs, 1);
    convertBufferToExpected(outputInterface, output, numberOfOutputs);

    return true;
}
/**
 * @brief ClusterLink::getOutput
 * @param hexagonName
 * @param output
 * @param numberOfOutputs
 * @return
 */
bool
ClusterLink::getOutput(const std::string& hexagonName,
                       float* output,
                       const uint64_t numberOfOutputs)
{
    auto it = m_cluster->outputInterfaces.find(hexagonName);
    if (it == m_cluster->outputInterfaces.end()) {
        return false;
    }

    OutputInterface* outputInterface = &it->second;
    convertOutputToBuffer(outputInterface, output, numberOfOutputs);

    return true;
}

/**
 * @brief ClusterLink::doTrain
 */
void
ClusterLink::doTrain()
{
    std::cout << "do train" << std::endl;
    m_cluster->startForwardCycle(false);
    m_cluster->finishCycle();
}

void
ClusterLink::doRequest()
{
    m_cluster->startForwardCycle(true);
    m_cluster->finishCycle();
}
