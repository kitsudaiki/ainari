/**
 * @file        hanami_core.cpp
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

#include "hanami_root.h"

#include <src/cluster/cluster.h>
#include <src/cluster/cluster_handler.h>
#include <src/cluster/cluster_init.h>
#include <src/common/files/binary_file.h>
#include <src/common/functions/file_functions.h>
#include <src/common/logger.h>
#include <src/common/threading/thread_handler.h>
#include <src/io/checkpoint/disc/checkpoint_io.h>
#include <src/processing/logical_host.h>
#include <src/processing/physical_host.h>
#include <src/thread_binder.h>

#include <iostream>

#include "cluster_link.h"

// init static variables
Hanami::GpuInterface* HanamiCore::gpuInterface = nullptr;
HanamiCore* HanamiCore::rootObj = nullptr;
PhysicalHost* HanamiCore::physicalHost = nullptr;

/**
 * @brief constructor
 */
HanamiCore::HanamiCore() { rootObj = this; }

/**
 * @brief destructor
 */
HanamiCore::~HanamiCore()
{
    // delete Hanami::Logger::m_logger;
}

/**
 * @brief init root-object
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
HanamiCore::init(const float maxMemoryUsage, std::string& errorMessage)
{
    if (m_isInit) {
        return false;
    }

    srand(time(NULL));

    // inti hosts
    Hanami::ErrorContainer error;
    physicalHost = new PhysicalHost(maxMemoryUsage);
    if (physicalHost->init(error) == false) {
        delete physicalHost;
        LOG_ERROR(error);
        errorMessage = error.toString();
        return false;
    }

    // create thread-binder
    if (ThreadBinder::getInstance()->init(error) == false) {
        error.addMessage("failed to init thread-binder");
        delete physicalHost;
        LOG_ERROR(error);
        errorMessage = error.toString();
        return false;
    }
    ThreadBinder::getInstance()->startThread();

    m_isInit = true;

    return true;
}

/**
 * @brief HanamiCore::createCluster
 * @param uuid
 * @param name
 * @param clusterTemplate
 * @param errorMessage
 * @return
 */
std::unique_ptr<ClusterLink>
HanamiCore::createCluster(const std::string& uuid,
                          const std::string& name,
                          const ClusterMeta& parsedCluster,
                          std::string& errorMessage)
{
    Hanami::ErrorContainer error;
    std::lock_guard<std::mutex> guard(m_clusterMutex);
    Cluster* newCluster = nullptr;

    do {
        // check if cluster already exist
        if (ClusterHandler::getInstance()->getCluster(uuid) != nullptr) {
            error.addMessage("Cluster with UUID '" + uuid + "' already exist.");
            break;
        }

        // create new cluster
        newCluster = new Cluster();
        if (newCluster->clusterHeader.name.setName(name) == false) {
            error.addMessage("New cluster-name '" + name
                             + "' too long, even this should be avoided by the API.");
            break;
        }

        // generate and initialize the cluster based on the cluster-templates
        if (newCluster->init(parsedCluster, uuid) == false) {
            error.addMessage("Failed to initialize cluster based on a template");
            break;
        }

        // add to cluster-handler
        if (ClusterHandler::getInstance()->addCluster(uuid, newCluster) == false) {
            error.addMessage("Failed to add cluster to cluster-handler.");
            break;
        }

        std::unique_ptr<ClusterLink> clusterLink(new ClusterLink(newCluster));
        return clusterLink;
    }
    while (true);

    // cleanup in case of a failure
    if (newCluster != nullptr) {
        delete newCluster;
    }
    errorMessage = error.toString();

    return std::unique_ptr<ClusterLink>{nullptr};
}

/**
 * @brief HanamiCore::deleteCluster
 * @param uuid
 * @return
 */
int
HanamiCore::deleteCluster(const std::string& uuid)
{
    std::lock_guard<std::mutex> guard(m_clusterMutex);

    // TODO: stop acitve task if necessasry
    if (ClusterHandler::getInstance()->removeCluster(uuid) == false) {
        return INVALID_INPUT;
    }

    return OK;
}

/**
 * @brief HanamiCore::createCheckpoint
 * @param clusterUuid
 * @param targetFilePath
 * @param errorMessage
 * @return
 */
int
HanamiCore::createCheckpoint(const std::string& clusterUuid,
                             const std::string& targetFilePath,
                             std::string& errorMessage)
{
    Hanami::ErrorContainer error;
    std::filesystem::path filePath = targetFilePath;

    std::lock_guard<std::mutex> guard(m_clusterMutex);

    Cluster* cluster = ClusterHandler::getInstance()->getCluster(clusterUuid);
    if (cluster == nullptr) {
        error.addMessage("Cluster with UUID '" + clusterUuid
                         + "'not found even it exists within the database");
        errorMessage = error.toString();
        return INVALID_INPUT;
    }

    // cluster->stateMachine
    for (Hexagon& hexagon : cluster->hexagons) {
        hexagon.attachedHost->syncWithHost(&hexagon);
    }
    CheckpointIO m_clusterIO;
    ReturnStatus ret = m_clusterIO.writeClusterToFile(*cluster, targetFilePath, error);
    if (ret != OK) {
        return ret;
    }

    return OK;
}

/**
 * @brief createRootObj
 * @return
 */
std::unique_ptr<HanamiCore>
createRootObj()
{
    return std::make_unique<HanamiCore>();
}
void
HanamiCore::my_cpp_function()
{
    std::cout << "my_cpp_function" << std::endl;
}

int
HanamiCore::area() const
{
    return 5 * 5;
}

int
HanamiCore::perimeter() const
{
    return 4 * 5;
}
