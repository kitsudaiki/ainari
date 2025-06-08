/**
 * @file        logical_host.h
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

#ifndef LOGICALHOST_H
#define LOGICALHOST_H

#include <processing/worker_thread.h>
#include <src/cluster/cluster.h>
#include <src/common/buffer/item_buffer.h>
#include <stdint.h>

#include <deque>
#include <vector>

class Cluster;

void handleClientOutput(Cluster* cluster);

class LogicalHost
{
   public:
    enum HostType {
        UNDEFINED_HOST_TYPE = 0,
        CPU_HOST_TYPE = 1,
        CUDA_HOST_TYPE = 2,
    };

    LogicalHost(const uint32_t localId);
    virtual ~LogicalHost();

    HostType getHostType() const;
    const std::string getUuid() const;

    virtual bool moveHexagon(Hexagon* hexagon) = 0;
    virtual void removeHexagon(Hexagon* hexagon) = 0;
    virtual void syncWithHost(Hexagon* hexagon) = 0;

    Hanami::ItemBuffer<Block> blocks;
    Hanami::ItemBuffer<SynapseSection> sections;

    void addWorkerTaskToQueue(const Hanami::WorkerTask task);
    const Hanami::WorkerTask getWorkerTaskFromQueue();

   protected:
    void continueAllThreads();

    uint64_t reductionCounter = 0;

    HostType m_hostType = UNDEFINED_HOST_TYPE;
    std::string m_uuid = "";
    uint32_t m_localId = 0;

    std::vector<WorkerThread*> m_workerThreads;
    std::deque<Hanami::WorkerTask> m_workerTaskQueue;
    std::mutex m_queue_lock;
};

#endif  // LOGICALHOST_H
