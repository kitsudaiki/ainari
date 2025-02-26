/**
 * @file       event_queue.h
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

#ifndef EVENTQUEUE_H
#define EVENTQUEUE_H

#include <hanami_common/threading/thread.h>

namespace Hanami
{

class EventQueue : public Hanami::Thread
{
   public:
    EventQueue(const std::string& threadName, const bool deleteEventObj);
    ~EventQueue();

   protected:
    void run();

   private:
    bool m_deleteEventObj = false;
};

}  // namespace Hanami

#endif  // EVENTQUEUE_H
