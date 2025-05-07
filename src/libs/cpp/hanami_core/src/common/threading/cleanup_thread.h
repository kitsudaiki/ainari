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

#ifndef KITSUNEMIMI_CLEANUP_THREAD_H
#define KITSUNEMIMI_CLEANUP_THREAD_H

#include <src/common/threading/thread.h>

#include <queue>

namespace Hanami
{

class CleanupThread : public Hanami::Thread
{
   public:
    static CleanupThread* getInstance();

    void addThreadForCleanup(Thread* thread);

   protected:
    void run();

   private:
    CleanupThread();
    ~CleanupThread();

    std::queue<Thread*> m_cleanupQueue;
    std::mutex m_mutex;
    static CleanupThread* m_cleanupThread;
};

}  // namespace Hanami

#endif  // KITSUNEMIMI_CLEANUP_THREAD_H
