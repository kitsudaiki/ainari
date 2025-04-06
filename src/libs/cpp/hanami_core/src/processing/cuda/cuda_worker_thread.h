/**
 * @file        cuda_worker_thread.h
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

#ifndef CUDAWORKERTHREAD_H
#define CUDAWORKERTHREAD_H

#include <hanami_common/threading/thread.h>
#include <processing/cuda/cuda_host.h>

class Cluster;

class CudaWorkerThread : public WorkerThread
{
   public:
    CudaWorkerThread(CudaHost* host);
    ~CudaWorkerThread();

   private:
    CudaHost* m_cudaHost = nullptr;
    void handleTrainForwardTask(Hanami::WorkerTask task);
    void handleTrainBackwardTask(Hanami::WorkerTask task);
    void handleReductionTask(const Hanami::WorkerTask task);
    void handleProcessTask(const Hanami::WorkerTask task);
};

#endif  // CUDAWORKERTHREAD_H
