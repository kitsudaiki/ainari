/**
 * @file       thread.cpp
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

#include <hanami_common/buffer/data_buffer.h>
#include <hanami_common/threading/cleanup_thread.h>
#include <hanami_common/threading/event.h>
#include <hanami_common/threading/thread.h>
#include <hanami_common/threading/thread_handler.h>

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param threadName global unique name of the thread for later identification
 */
Thread::Thread(const std::string& threadName)
    : m_threadName(threadName), m_threadId(ThreadHandler::getInstance()->getNewId())
{
}

/**
 * @brief destructor
 */
Thread::~Thread()
{
    ThreadHandler::getInstance()->unregisterThread(m_threadName, m_threadId);
    stopThread();
    clearEventQueue();

    delete m_thread;
}

/**
 * @brief bind thread to a list of core-ids
 *
 * @param coreIds list with core-ids to bind to
 *
 * @return false if precheck of bind failed, else true
 */
bool
Thread::bindThreadToCores(const std::vector<uint64_t> coreIds)
{
    // bind thread
    cpu_set_t cpuset;
    CPU_ZERO(&cpuset);
    for (const uint64_t coreId : coreIds) {
        CPU_SET(coreId, &cpuset);
    }
    if (pthread_setaffinity_np(m_thread->native_handle(), sizeof(cpu_set_t), &cpuset) != 0) {
        return false;
    }

    m_coreIds = coreIds;

    return true;
}

/**
 * @brief bind the thread to a specific cpu-thread
 *
 * @param coreId id of the cpu-thread where bind
 *
 * @return false if precheck of bind failed, else true
 */
bool
Thread::bindThreadToCore(const uint64_t coreId)
{
    // bind thread
    cpu_set_t cpuset;
    CPU_ZERO(&cpuset);
    CPU_SET(coreId, &cpuset);
    if (pthread_setaffinity_np(m_thread->native_handle(), sizeof(cpu_set_t), &cpuset) != 0) {
        return false;
    }

    m_coreIds.clear();
    m_coreIds.push_back(coreId);

    return true;
}

/**
 * @brief get the id of the core where the thread is bound to
 *
 * @return -1 if no core defined, else core-id, on which the thread in bound
 */
const std::vector<uint64_t>
Thread::getCoreIds() const
{
    return m_coreIds;
}

/**
 * @brief get name of the thread
 *
 * @return name of the thread
 */
const std::string
Thread::getThreadName() const
{
    return m_threadName;
}

/**
 * @brief Thread::getThreadId
 * @return
 */
uint64_t
Thread::getThreadId() const
{
    return m_threadId;
}

/**
 * @brief add a new event to the queue
 * @param newEvent new event
 */
void
Thread::addEventToQueue(Event* newEvent)
{
    // precheck
    if (m_active == false) {
        return;
    }

    // add new event to queue
    while (m_eventQueue_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }
    m_eventQueue.push_back(newEvent);
    m_eventQueue_lock.clear(std::memory_order_release);
}

/**
 * @brief get the next event from the queue
 *
 * @return nullptr, if the queue is empty, else the next event of the queue
 */
Event*
Thread::getEventFromQueue()
{
    Event* result = nullptr;

    while (m_eventQueue_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    // get the next from the queue
    if (m_eventQueue.empty() == false) {
        result = m_eventQueue.front();
        m_eventQueue.pop_front();
    }

    m_eventQueue_lock.clear(std::memory_order_release);

    return result;
}

/**
 * @brief start the thread
 *
 * @return false if already running, else true
 */
bool
Thread::startThread()
{
    // precheck
    if (m_active) {
        return false;
    }

    // init new thread
    ThreadHandler::getInstance()->registerThread(this);
    m_abort = false;
    m_active = true;
    m_thread = new std::thread(&Thread::run, this);

    return true;
}

/**
 * @brief give thread in case of destruction to a cleanup-thread, because a thread can't delete
 *        itself
 *
 * @return false, if already scheduled for deletion, else true
 */
bool
Thread::scheduleThreadForDeletion()
{
    // check and set deletion-flag
    while (m_eventQueue_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }
    if (m_scheduledForDeletion) {
        m_eventQueue_lock.clear(std::memory_order_release);
        return false;
    }
    m_scheduledForDeletion = true;
    m_eventQueue_lock.clear(std::memory_order_release);

    // give to cleanup-thread for later deletion
    CleanupThread::getInstance()->addThreadForCleanup(this);

    return true;
}

/**
 * @brief stop a thread without killing the thread
 */
void
Thread::stopThread()
{
    // precheck
    if (m_active == false) {
        return;
    }

    // give thread abort-flag and wait until its end
    m_abort = true;

    if (m_thread->joinable()) {
        m_thread->join();
    }

    m_active = false;
}

/**
 * @brief delete all event-objects within the event-queue
 */
void
Thread::clearEventQueue()
{
    while (m_eventQueue_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    while (m_eventQueue.empty() == false) {
        Event* obj = m_eventQueue.front();
        m_eventQueue.pop_front();
        delete obj;
    }

    m_eventQueue_lock.clear(std::memory_order_release);
}

/**
 * @brief say the thread, he should wait at the next barrier
 */
void
Thread::initBlockThread()
{
    m_block = true;
}

/**
 * @brief let the thread continue if he waits at the barrier
 */
void
Thread::continueThread()
{
    m_cv.notify_one();
}

/**
 * @brief spin-lock
 */
void
Thread::spinLock()
{
    while (m_spin_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
        /**
         * Explaination from stack overflow:
         *
         * What's more, if you use volatile, gcc will store those variables in RAM and add a bunch
         * of ldd and std to copy them to temporary registers.
         * This approach, on the other hand, doesn't use volatile and generates no such overhead.
         */
    }
}

/**
 * @brief spin-unlock
 */
void
Thread::spinUnlock()
{
    m_spin_lock.clear(std::memory_order_release);
}

/**
 * @brief let the thread wait at a barrier
 */
void
Thread::blockThread()
{
    m_block = true;
    std::unique_lock<std::mutex> lock(m_cvMutex);
    m_cv.wait(lock);
    m_block = false;
}

/**
 * @brief let the thread sleep for a specific amount of microseconds
 */
void
Thread::sleepThread(const uint32_t microSeconds)
{
    std::this_thread::sleep_for(std::chrono::microseconds(microSeconds));
}

/**
 * @brief check if thread is active
 *
 * @return true, if active, else false
 */
bool
Thread::isActive() const
{
    return m_active;
}

}  // namespace Hanami
