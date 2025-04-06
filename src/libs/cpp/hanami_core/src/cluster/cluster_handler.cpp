/**
 * @file        cluster_handler.cpp
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

#include "cluster_handler.h"

#include <cluster/cluster.h>

ClusterHandler* ClusterHandler::instance = nullptr;

/**
 * @brief constructor
 */
ClusterHandler::ClusterHandler() {}

/**
 * @brief add new cluster to handler
 *
 * @param uuid uuid of the cluster
 * @param newCluster pointer to cluster-object
 *
 * @return false if uuid is already registered, else true
 */
bool
ClusterHandler::addCluster(const std::string uuid, Cluster* newCluster)
{
    return m_allCluster.try_emplace(uuid, newCluster).second;
}

/**
 * @brief remove a cluster from the handler
 *
 * @param uuid uuid of the cluster
 *
 * @return false, if uuid was not found, else true
 */
bool
ClusterHandler::removeCluster(const std::string uuid)
{
    const auto it = m_allCluster.find(uuid);
    if (it != m_allCluster.end()) {
        if (it->second != nullptr) {
            // unregister cluster in websocket and close websocket, if direct connection is active
            /*if (it->second->msgClient != nullptr) {
                it->second->msgClient->m_targetCluster = nullptr;
                Hanami::ErrorContainer error;
                //it->second->msgClient->closeClient();
            }*/

            delete it->second;
        }
        m_allCluster.erase(it);
        return true;
    }

    return false;
}

/**
 * @brief request a specific cluster from the handler
 *
 * @param uuid uuid of the cluster
 *
 * @return pointer to the requested cluster
 */
Cluster*
ClusterHandler::getCluster(const std::string uuid)
{
    const auto it = m_allCluster.find(uuid);
    if (it != m_allCluster.end()) {
        return it->second;
    }

    return nullptr;
}
