/**
 * @file       barrier.h
 *
 *  @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *  @copyright  Apache License Version 2.0
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

#ifndef BARRIER_H
#define BARRIER_H

#include <pthread.h>
#include <unistd.h>

#include <atomic>
#include <condition_variable>
#include <mutex>

namespace Hanami
{

class Barrier
{
   public:
    Barrier(const uint32_t numberOfThreads);

    void triggerBarrier();
    void releaseAll();

   private:
    std::mutex m_mutex;
    std::condition_variable m_cond;
    std::atomic_flag m_spin_lock = ATOMIC_FLAG_INIT;

    uint32_t m_numberOfThreads = 0;
    uint32_t m_counter = 0;
    bool m_abort = false;
};

}  // namespace Hanami

#endif  // BARRIER_H
