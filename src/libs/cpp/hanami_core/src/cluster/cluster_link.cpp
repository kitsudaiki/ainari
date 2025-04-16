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

#include <iostream>

ClusterLink::ClusterLink(Cluster* cluster) { m_cluster = cluster; }

ClusterLink::~ClusterLink()
{
    if (m_cluster != nullptr) {
        delete m_cluster;
    }
}

void
ClusterLink::printMetrics() const
{
    std::cout << "Metrics of cluster " << m_cluster->clusterHeader.uuid.toString() << ": "
              << std::endl;
    std::cout << "    Number of hexagons: " << m_cluster->hexagons.size() << std::endl;
}
