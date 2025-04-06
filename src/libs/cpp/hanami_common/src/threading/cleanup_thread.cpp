/**
 * @file       thread_handler.h
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

#include <arpa/inet.h>
#include <hanami_common/threading/cleanup_thread.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

namespace Hanami
{

CleanupThread* CleanupThread::m_cleanupThread = nullptr;

/**
 * constructor
 */
CleanupThread::CleanupThread() : Hanami::Thread("Kitsunemimi_CleanupThread") {}

/**
 * @brief destructor
 */
CleanupThread::~CleanupThread() {}

/**
 * @brief static methode to get instance of the interface
 *
 * @return pointer to the static instance
 */
CleanupThread*
CleanupThread::getInstance()
{
    if (m_cleanupThread == nullptr) {
        m_cleanupThread = new CleanupThread();
        m_cleanupThread->startThread();
    }
    return m_cleanupThread;
}

/**
 * @brief schedule a thread for delteion
 * @param thread thread, which should be deleted
 */
void
CleanupThread::addThreadForCleanup(Thread* thread)
{
    m_mutex.lock();
    m_cleanupQueue.push(thread);
    m_mutex.unlock();
}

/**
 * @brief loop, which tries to delete all thread
 */
void
CleanupThread::run()
{
    while (m_abort == false) {
        sleepThread(100000);

        m_mutex.lock();
        if (m_cleanupQueue.size() > 0) {
            Thread* thread = m_cleanupQueue.front();
            m_cleanupQueue.pop();
            delete thread;
        }
        m_mutex.unlock();
    }
}

}  // namespace Hanami
