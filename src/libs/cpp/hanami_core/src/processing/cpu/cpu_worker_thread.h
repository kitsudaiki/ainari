/**
 * @file        cpu_worker_thread.h
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

#ifndef CPUWORKERTHREAD_H
#define CPUWORKERTHREAD_H

#include <processing/worker_thread.h>

class Cluster;
class CpuHost;

class CpuWorkerThread : public WorkerThread
{
   public:
    CpuWorkerThread(CpuHost* host);
    ~CpuWorkerThread();

   private:
    void handleTrainForwardTask(Hanami::WorkerTask task);
    void handleTrainBackwardTask(Hanami::WorkerTask task);
    void handleReductionTask(const Hanami::WorkerTask task);
    void handleProcessTask(const Hanami::WorkerTask task);
};

#endif  // CPUWORKERTHREAD_H
