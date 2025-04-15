/**
 * @file        cluster_init.cpp
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

#include "cluster_init.h"

#include <cluster/cluster.h>
#include <cluster/cluster_init.h>
#include <cluster/objects.h>
#include <hanami_common/logger.h>
#include <include/hanami_core/hanami_root.h>
#include <processing/physical_host.h>
#include <routing_functions.h>

/**
 * @brief calculate the number of blocks for a specific number of neurons
 *
 * @param numberOfNeurons number of neurons
 *
 * @return number of neuron-blocks
 */
uint32_t
calcNumberOfNeuronBlocks(const uint32_t numberOfNeurons)
{
    uint32_t numberOfSections = numberOfNeurons / NEURONS_PER_BLOCK;
    if (numberOfNeurons % NEURONS_PER_BLOCK != 0) {
        numberOfSections++;
    }

    return numberOfSections;
}

/**
 * @brief initalize new cluster
 *
 * @param cluster pointer to the uninitionalized cluster
 * @param clusterMeta parsed data from the cluster-template
 * @param uuid uuid for the new cluster
 * @param host initial host to attach the hexagons. if nullptr, use the first cpu-host
 *
 * @return true, if successful, else false
 */
bool
initNewCluster(Cluster* cluster,
               const ClusterMeta& clusterMeta,
               const std::string& uuid,
               LogicalHost* host)
{
    initializeHeader(cluster, uuid);
    initializeSettings(cluster, clusterMeta);
    initializeHexagons(cluster, clusterMeta, host);
    initializeInputs(cluster, clusterMeta);
    initializeOutputs(cluster, clusterMeta);

    return true;
}

/**
 * @brief create a new header for a cluster
 *
 * @param cluster pointer to cluster
 * @param uuid uuid of the cluster
 */
void
initializeHeader(Cluster* cluster, const std::string& uuid)
{
    cluster->clusterHeader = ClusterHeader();
    strncpy(cluster->clusterHeader.uuid.uuid, uuid.c_str(), uuid.size());
}

/**
 * @brief initialize settings block of a cluster
 *
 * @param cluster pointer to cluster
 * @param clusterMeta meta-data of cluster-template with the new cluster-settings
 */
void
initializeSettings(Cluster* cluster, const ClusterMeta& clusterMeta)
{
    cluster->clusterHeader.settings.neuronCooldown = clusterMeta.neuronCooldown;
    cluster->clusterHeader.settings.refractoryTime = clusterMeta.refractoryTime;
    cluster->clusterHeader.settings.maxConnectionDistance = clusterMeta.maxConnectionDistance;
}

/**
 * @brief initialize input-neurons of a cluster
 *
 * @param cluster pointer to cluster
 * @param clusterMeta meta-data of cluster-template with information
 */
void
initializeInputs(Cluster* cluster, const ClusterMeta& clusterMeta)
{
    for (const InputMeta& inputMeta : clusterMeta.inputs) {
        InputInterface inputInterface;
        inputInterface.targetHexagonId = inputMeta.targetHexagonId;
        inputInterface.name = inputMeta.name;

        cluster->inputInterfaces.try_emplace(inputMeta.name, inputInterface);

        Hexagon* targetHexagon = &cluster->hexagons[inputInterface.targetHexagonId];
        targetHexagon->header.isInputHexagon = true;
        targetHexagon->inputInterface = &cluster->inputInterfaces[inputMeta.name];
    }
}

/**
 * @brief initialize output-neurons of a cluster
 *
 * @param cluster pointer to cluster
 * @param clusterMeta meta-data of cluster-template with information
 */
void
initializeOutputs(Cluster* cluster, const ClusterMeta& clusterMeta)
{
    for (const OutputMeta& outputMeta : clusterMeta.outputs) {
        OutputInterface outputInterface;
        outputInterface.targetHexagonId = outputMeta.targetHexagonId;
        outputInterface.name = outputMeta.name;
        outputInterface.type = outputMeta.type;

        cluster->outputInterfaces.try_emplace(outputMeta.name, outputInterface);

        cluster->hexagons[outputMeta.targetHexagonId].header.isOutputHexagon = true;
        cluster->hexagons[outputMeta.targetHexagonId].outputInterface
            = &cluster->outputInterfaces[outputMeta.name];
    }
}

/**
 * @brief init all hexagons
 *
 * @param cluster pointer to cluster
 * @param clusterMeta meta-data of cluster-template with the new cluster-settings
 * @param host initial host to attach the hexagons. if nullptr, use the first cpu-host
 */
void
initializeHexagons(Cluster* cluster, const ClusterMeta& clusterMeta, LogicalHost* host)
{
    cluster->hexagons.resize(clusterMeta.hexagons.size());

    for (uint32_t i = 0; i < clusterMeta.hexagons.size(); i++) {
        Hexagon* newHexagon = &cluster->hexagons[i];
        newHexagon->cluster = cluster;
        newHexagon->header.hexagonId = i;
        newHexagon->header.axonTarget = i;
        newHexagon->header.hexagonPos = clusterMeta.hexagons.at(i);
        if (host != nullptr) {
            newHexagon->attachedHost = host;
        }
        else {
            newHexagon->attachedHost = HanamiCore::physicalHost->getFirstHost();
        }
        std::fill_n(newHexagon->neighbors, 12, UNINIT_STATE_32);
    }

    updateAxons(cluster, clusterMeta);
    connectAllHexagons(cluster);
    initializeTargetHexagonList(cluster);

    return;
}

/**
 * @brief update axons of the hexagon
 *
 * @param cluster pointer to cluster
 * @param clusterMeta meta-data of cluster-template with axon-information
 */
void
updateAxons(Cluster* cluster, const ClusterMeta& clusterMeta)
{
    for (const AxonMeta& axonMeta : clusterMeta.axons) {
        cluster->hexagons[axonMeta.sourceId].header.axonTarget = axonMeta.targetId;
    }
}

/**
 * @brief connect a single side of a specific hexagon of a cluster
 *
 * @param cluster pointer to cluster
 * @param sourceHexagon pointer to the hexagon
 * @param side side of the hexagon to connect
 */
void
connectHexagon(Cluster* cluster, Hexagon& sourceHexagon, const uint8_t side)
{
    const Hanami::Position next = getNeighborPos(sourceHexagon.header.hexagonPos, side);
    // debug-output
    // std::cout<<next.x<<" : "<<next.y<<" : "<<next.z<<std::endl;

    if (next.isValid()) {
        for (Hexagon& targetHexagon : cluster->hexagons) {
            if (targetHexagon.header.hexagonPos == next) {
                sourceHexagon.neighbors[side] = targetHexagon.header.hexagonId;
                targetHexagon.neighbors[11 - side] = sourceHexagon.header.hexagonId;
            }
        }
    }
}

/**
 * @brief connect all breaks of the cluster
 */
void
connectAllHexagons(Cluster* cluster)
{
    for (Hexagon& sourceHexagon : cluster->hexagons) {
        for (uint8_t side = 0; side < 12; side++) {
            connectHexagon(cluster, sourceHexagon, side);
        }
    }
}

/**
 * @brief get next possible hexagon
 *
 * @param cluster pointer to cluster
 * @param currentHexagon actual hexagon
 * @param maxPathLength maximum path length left
 * @param sourceHexagonId id of the source-hexagon, which initialized the search
 *
 * @return last hexagon-id of the gone path
 */
uint32_t
goToNextInitHexagon(Cluster* cluster,
                    Hexagon* currentHexagon,
                    int32_t& maxPathLength,
                    const uint32_t sourceHexagonId)
{
    // check path-length to not go too far
    maxPathLength--;
    if (maxPathLength <= 0 && currentHexagon->header.hexagonId != sourceHexagonId) {
        return currentHexagon->header.hexagonId;
    }

    // check based on the chance, if you go to the next, or not
    const float chanceForNext = 0.5f;  // TODO: make hard-coded value configurable
    if (1000.0f * chanceForNext > (rand() % 1000)
        && currentHexagon->header.hexagonId != sourceHexagonId)
    {
        return currentHexagon->header.hexagonId;
    }

    // filter all avaialable next hexagons
    // TODO: require a better solution, which doesn't need to filter this every time
    std::vector<uint32_t> availableNext;
    const uint8_t possibleNextSides[7] = {9, 3, 1, 4, 11, 5, 2};
    for (uint32_t i = 0; i < 7; i++) {
        const uint8_t side = possibleNextSides[i];
        const uint32_t nextHexagonId = currentHexagon->neighbors[side];
        if (nextHexagonId != UNINIT_STATE_32) {
            availableNext.push_back(nextHexagonId);
        }
    }

    // handle end of the path
    if (availableNext.size() == 0) {
        return currentHexagon->header.hexagonId;
    }

    // select one of the filtered results
    const uint32_t selectedNext = availableNext[rand() % availableNext.size()];
    return goToNextInitHexagon(
        cluster, &cluster->hexagons[selectedNext], maxPathLength, sourceHexagonId);
}

/**
 * @brief init target-hexagon-list of all hexagons
 *
 * @param cluster pointer to cluster
 */
void
initializeTargetHexagonList(Cluster* cluster)
{
    for (Hexagon& hexagon : cluster->hexagons) {
        std::vector<uint32_t> availableNext;
        for (uint32_t counter = 0; counter < NUMBER_OF_POSSIBLE_NEXT; counter++) {
            int32_t maxPathLength = cluster->clusterHeader.settings.maxConnectionDistance;
            Hexagon* baseHexagon = &cluster->hexagons[hexagon.header.axonTarget];
            const uint32_t targetHexagonId = goToNextInitHexagon(
                cluster, baseHexagon, maxPathLength, hexagon.header.hexagonId);
            if (hexagon.header.hexagonId != targetHexagonId) {
                hexagon.possibleHexagonTargetIds[counter] = targetHexagonId;
            }
            else {
                hexagon.possibleHexagonTargetIds[counter] = UNINIT_STATE_32;
            }
        }
    }
}
