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

#include <cluster/cluster_init.h>
#include <processing/cuda/cuda_functions.h>
#include <processing/logical_host.h>
#include <src/cluster/cluster.h>
#include <src/common/threading/thread.h>
#include <src/io/checkpoint/disc/checkpoint_io.h>

#include <filesystem>
#include <iostream>

/**
 * @brief constructor
 */
Cluster::Cluster() { m_counter.store(0, std::memory_order_relaxed); }

/**
 * @brief constructor to create cluster from a checkpoint
 *
 * @param data pointer to data with checkpoint
 * @param dataSize size of checkpoint in number of bytes
 */
// Cluster::Cluster(const void* data, const uint64_t dataSize)
//{
//     m_counter.store(0, std::memory_order_relaxed);
// }

/**
 * @brief destructor
 */
Cluster::~Cluster()
{
    for (Hexagon& hexagon : hexagons) {
        hexagon.attachedHost->removeHexagon(&hexagon);
    }
}

/**
 * @brief Cluster::incrementAndCompare
 * @param referenceValue
 */
bool
Cluster::incrementAndCompare(const uint32_t referenceValue)
{
    const int incrementedValue = m_counter.fetch_add(1, std::memory_order_relaxed);
    if (incrementedValue == referenceValue - 1) {
        m_counter.store(0, std::memory_order_relaxed);
        return true;
    }

    return false;
}

/**
 * @brief get uuid of the cluster
 *
 * @return uuid of the cluster
 */
const std::string
Cluster::getUuid()
{
    return clusterHeader.uuid.toString();
}

/**
 * @brief init new cluster
 *
 * @param clusterTemplate meta-data read from a cluster-template
 * @param uuid uuid of the cluster
 * @param host initial host to attach the hexagons. if nullptr, use the first cpu-host (default:
 * nullptr)
 *
 * @return true, if successful, else false
 */
bool
Cluster::init(const ClusterMeta& clusterTemplate, const std::string& uuid, LogicalHost* host)
{
    return initNewCluster(this, clusterTemplate, uuid, host);
}

/**
 * @brief start a new forward train-cycle
 */
void
Cluster::startForwardCycle(const bool runNormalMode)
{
    Hanami::WorkerTask task;
    task.cluster = this;
    task.hexagonId = 0;
    task.blockId = UNINIT_STATE_16;
    task.mode = ClusterProcessingMode::TRAIN_FORWARD_MODE;
    if (runNormalMode) {
        task.mode = ClusterProcessingMode::NORMAL_MODE;
    }
    hexagons.front().attachedHost->addWorkerTaskToQueue(task);
}

/**
 * @brief start a new backward train-cycle
 */
void
Cluster::startBackwardCycle()
{
    Hanami::WorkerTask task;
    task.cluster = this;
    task.hexagonId = hexagons.size() - 1;
    task.blockId = UNINIT_STATE_16;
    task.mode = ClusterProcessingMode::TRAIN_BACKWARD_MODE;
    hexagons.back().attachedHost->addWorkerTaskToQueue(task);
}

/**
 * @brief update state of the cluster, which is caled for each finalized cluster
 */
void
Cluster::updateClusterState(const Hanami::WorkerTask& task)
{
    std::lock_guard<std::mutex> guard(m_clusterStateLock);

    // TODO (kitsudaiki): check why this flag behave a bit strange
    // enableCreation = false;

    // trigger next lerning phase, if already in phase 1
    if (task.mode == ClusterProcessingMode::TRAIN_FORWARD_MODE) {
        startBackwardCycle();
    }
    else if (task.mode == ClusterProcessingMode::TRAIN_BACKWARD_MODE) {
        // sendClusterTrainEndMessage(this);
        //  countSynapses(*this);
        finishCycle();
        return;
    }
    else if (task.mode == ClusterProcessingMode::NORMAL_MODE) {
        // sendClusterNormalEndMessage(this);
        finishCycle();
        return;
    }
}

/**
 * @brief Cluster::finishCycle
 * @return
 */
void
Cluster::finishCycle()
{
    m_barrier.triggerBarrier();
}

/**
 * @brief create checkpoint of the cluster
 *
 * @param targetFilePath local file-path, where to store the resulting checkpoint
 *
 * @return return-status
 */
ReturnStatus
Cluster::createCheckpoint(const std::string& targetFilePath)
{
    std::string error;
    std::filesystem::path filePath = targetFilePath;

    for (Hexagon& hexagon : this->hexagons) {
        hexagon.attachedHost->syncWithHost(&hexagon);
    }
    // HINT (kitsudaiki): has to be on the heap, instead on object on the stack
    // or otherwise the autocxx will fail when calling this function. I have no idea why,
    // but at least this workarounc works fine.
    std::unique_ptr<CheckpointIO> clusterIO = std::make_unique<CheckpointIO>();
    ReturnStatus ret = clusterIO->writeClusterToFile(*this, targetFilePath, error);
    if (ret != OK) {
        std::cout << "error: " << error << std::endl;
        return ret;
    }

    return OK;
}

/**
 * @brief restore a cluster from a checkpoint-file
 *
 * @param targetFilePath path to the local checkpoint-file, which should be restored
 *
 * @return return-status
 */
ReturnStatus
Cluster::restoreCheckpoint(const std::string& targetFilePath)
{
    std::string error;
    std::filesystem::path filePath = targetFilePath;

    for (Hexagon& hexagon : hexagons) {
        hexagon.attachedHost->syncWithHost(&hexagon);
    }
    // HINT (kitsudaiki): has to be on the heap, instead on object on the stack
    // or otherwise the autocxx will fail when calling this function. I have no idea why,
    // but at least this workarounc works fine.
    std::unique_ptr<CheckpointIO> clusterIO = std::make_unique<CheckpointIO>();
    ReturnStatus ret = clusterIO->restoreClusterFromFile(*this, targetFilePath, error);
    if (ret != OK) {
        std::cout << "error: " << error << std::endl;
        return ret;
    }

    return OK;
}
