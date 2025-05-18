/**
 * @file        cluster.h
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

#ifndef HANAMI_CLUSTER_H
#define HANAMI_CLUSTER_H

#include <src/cluster/objects.h>
#include <src/common/threading/barrier.h>

#include <atomic>
#include <map>
#include <mutex>

#include "hanami_structs.h"

class LogicalHost;
namespace Hanami
{
struct WorkerTask;
}

class Cluster
{
   public:
    Cluster();
    // Cluster(const void* data, const uint64_t dataSize);
    ~Cluster();

    // cluster-data
    ClusterHeader clusterHeader;
    ClusterMetrics metrics;

    std::vector<Hexagon> hexagons;
    std::map<std::string, InputInterface> inputInterfaces;
    std::map<std::string, OutputInterface> outputInterfaces;

    // meta
    const std::string getUuid();
    bool init(const ClusterMeta& clusterTemplate,
              const std::string& uuid,
              LogicalHost* host = nullptr);

    // states
    bool goToNextState(const uint32_t nextStateId);
    void startForwardCycle(const bool runNormalMode);
    void startBackwardCycle();
    void updateClusterState(const Hanami::WorkerTask& task);

    // counter for parallel-processing
    bool incrementAndCompare(const uint32_t referenceValue);
    void finishCycle();

    ReturnStatus createCheckpoint(const std::string& targetFilePath);
    ReturnStatus restoreCheckpoint(const std::string& targetFilePath);

   private:
    std::mutex m_clusterStateLock;
    std::atomic<int> m_counter;
    Hanami::Barrier m_barrier = Hanami::Barrier(2);
};

#endif  // HANAMI_CLUSTER_H
