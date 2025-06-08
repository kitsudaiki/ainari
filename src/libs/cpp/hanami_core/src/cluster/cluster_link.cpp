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
#include <src/processing/logical_host.h>

/**
 * @brief constructor
 *
 * @param cluster pointer to the cluster related to this specific link
 */
ClusterLink::ClusterLink(Cluster* cluster) { m_cluster = cluster; }

ClusterLink::~ClusterLink()
{
    if (m_cluster != nullptr) {
        delete m_cluster;
    }
}

/**
 * @brief create checkpoint of the cluster
 *
 * @param targetFilePath local file-path, where to store the resulting checkpoint
 *
 * @return return-status
 */
int
ClusterLink::createCheckpoint(const std::string& targetFilePath)
{
    return m_cluster->createCheckpoint(targetFilePath);
}

/**
 * @brief restore a cluster from a checkpoint-file
 *
 * @param targetFilePath path to the local checkpoint-file, which should be restored
 *
 * @return return-status
 */
int
ClusterLink::restoreCheckpoint(const std::string& targetFilePath)
{
    return m_cluster->restoreCheckpoint(targetFilePath);
}

/**
 * @brief fill input-hexagons with values
 *
 * @param hexagonName name of the hexagon
 * @param input pointer to the list with the input-values
 * @param numberOfInputs number of input-values in the list
 * @param bufferOffset offset within the input-buffer
 * @param totalTimeLength total time-length to calculate the correct size of the buffer
 *
 * @return false, if hexagon with the name doesn't exist, else true
 */
bool
ClusterLink::fillInput(const std::string& hexagonName,
                       const float* input,
                       const uint64_t numberOfInputs,
                       const uint64_t bufferOffset,
                       const uint64_t totalTimeLength)
{
    auto it = m_cluster->inputInterfaces.find(hexagonName);
    if (it == m_cluster->inputInterfaces.end()) {
        return false;
    }

    InputInterface* inputInterface = &it->second;
    inputInterface->initBuffer(numberOfInputs, totalTimeLength);

    AxonBlock* axonBlock = nullptr;
    Axon* axon = nullptr;
    uint64_t blockId = 0;
    uint16_t axonId = 0;
    uint64_t counter = bufferOffset;
    float val = 0.0f;

    for (uint64_t i = 0; i < numberOfInputs; ++i) {
        val = input[i];
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
 * @brief fill output-hexagons with expect-values
 *
 * @param hexagonName name of the hexagon
 * @param output pointer to the list with the expected output-values
 * @param numberOfOutputs number of output-values in the list
 *
 * @return false, if hexagon with the name doesn't exist, else true
 */
bool
ClusterLink::fillExpected(const std::string& hexagonName,
                          const float* output,
                          const uint64_t numberOfOutputs)
{
    auto it = m_cluster->outputInterfaces.find(hexagonName);
    if (it == m_cluster->outputInterfaces.end()) {
        return false;
    }

    OutputInterface* outputInterface = &it->second;
    outputInterface->initBuffer(numberOfOutputs);
    convertBufferToExpected(outputInterface, output, numberOfOutputs);

    return true;
}

/**
 * @brief get the size of an output-hexagon
 *
 * @param hexagonName name of the hexagon
 *
 * @return number of outputs of the hexagon
 */
uint64_t
ClusterLink::getOutputHexagonSize(const std::string& hexagonName)
{
    auto it = m_cluster->outputInterfaces.find(hexagonName);
    if (it == m_cluster->outputInterfaces.end()) {
        return UNINIT_STATE_64;
    }

    return it->second.outputNeurons.size();
}

/**
 * @brief get resulting values of an output-hexagon
 *
 * @param hexagonName name of the hexagon
 * @param output pointer to the target-list
 * @param numberOfOutputs size of the list
 *
 * @return false, if hexagon with the name doesn't exist, else true
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
 * @brief get number of values of a hexagon
 *
 * @param hexagonName name of the hexagon
 *
 * @return size of the hexagon
 */
uint64_t
ClusterLink::getSize(const std::string& hexagonName)
{
    auto it = m_cluster->outputInterfaces.find(hexagonName);
    if (it == m_cluster->outputInterfaces.end()) {
        return 0;
    }

    return it->second.outputNeurons.size();
}

/**
 * @brief start a train-cycle
 */
void
ClusterLink::doTrain()
{
    m_cluster->startForwardCycle(false);
    m_cluster->finishCycle();
}

/**
 * @brief start a request-cycle
 */
void
ClusterLink::doRequest()
{
    m_cluster->startForwardCycle(true);
    m_cluster->finishCycle();
}
