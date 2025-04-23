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
