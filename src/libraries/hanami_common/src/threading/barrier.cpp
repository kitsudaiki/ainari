/**
 * @file       barrier.cpp
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
#include <hanami_common/threading/barrier.h>

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param numberOfThreads number of threads to wait at the barrier
 */
Barrier::Barrier(const uint32_t numberOfThreads)
{
    m_numberOfThreads = numberOfThreads;
    m_counter = numberOfThreads;
}

/**
 * @brief Hold at the barrier or release all, if the required number of waiting threads not readed
 *        of else release all waiting threads
 */
void
Barrier::triggerBarrier()
{
    while (m_spin_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    m_counter--;
    if (m_counter == 0) {
        m_counter = m_numberOfThreads;
        m_spin_lock.clear(std::memory_order_release);
        usleep(1);
        m_cond.notify_all();
    }
    else {
        std::unique_lock<std::mutex> lock(m_mutex);
        m_spin_lock.clear(std::memory_order_release);
        m_cond.wait(lock);
    }

    return;
}

/**
 * @brief release all waiting threads at the barrier, to avoid dead-locks
 */
void
Barrier::releaseAll()
{
    m_cond.notify_all();
    m_counter = m_numberOfThreads;
}

}  // namespace Hanami
