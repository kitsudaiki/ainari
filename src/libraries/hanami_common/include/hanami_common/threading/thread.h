/**
 * @file       thread.h
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

#ifndef THREAD_H
#define THREAD_H

#include <pthread.h>

#include <atomic>
#include <condition_variable>
#include <deque>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

namespace Hanami
{
struct DataBuffer;
class Event;
class ThreadHandler;

class Thread
{
   public:
    Thread(const std::string& threadName);
    virtual ~Thread();

    bool startThread();

    bool scheduleThreadForDeletion();

    void continueThread();
    void initBlockThread();

    bool isActive() const;
    bool bindThreadToCores(const std::vector<uint64_t> coreIds);
    bool bindThreadToCore(const uint64_t coreId);
    const std::vector<uint64_t> getCoreIds() const;
    const std::string getThreadName() const;
    uint64_t getThreadId() const;

    void addEventToQueue(Event* newEvent);
    void clearEventQueue();

   protected:
    bool m_abort = false;
    bool m_block = false;

    // lock functions
    void blockThread();
    void sleepThread(const uint32_t microSeconds);
    void spinLock();
    void spinUnlock();

    // event-queue
    Event* getEventFromQueue();

    virtual void run() = 0;

   private:
    void stopThread();

    // generial variables
    std::thread* m_thread = nullptr;
    const std::string m_threadName = "";
    const uint64_t m_threadId = 0;

    bool m_active = false;
    bool m_scheduledForDeletion = false;
    std::vector<uint64_t> m_coreIds;

    // event-queue-variables
    std::deque<Event*> m_eventQueue;
    std::atomic_flag m_eventQueue_lock = ATOMIC_FLAG_INIT;

    // lock variables
    std::atomic_flag m_spin_lock = ATOMIC_FLAG_INIT;
    std::mutex m_cvMutex;
    std::condition_variable m_cv;
};

}  // namespace Hanami

#endif  // THREAD_H
