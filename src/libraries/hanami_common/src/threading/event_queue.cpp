/**
 * @file       event_queue.cpp
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

#include <hanami_common/logger.h>
#include <hanami_common/threading/event.h>
#include <hanami_common/threading/event_queue.h>

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param deleteEventObj set to true, to delete the event-object after it was processed
 */
EventQueue::EventQueue(const std::string& threadName, const bool deleteEventObj)
    : Thread(threadName)
{
    m_deleteEventObj = deleteEventObj;
}

/**
 * @brief destructor
 */
EventQueue::~EventQueue()
{
    // Workaround. When the events in the queue are not allowed to be deleted, because they are
    // deleted somewhere else, the list has to be cleared before calling the destructor
    // of the parent thread-class.
    if (m_deleteEventObj == false) {
        Event* event = nullptr;
        do {
            event = getEventFromQueue();
        }
        while (event != nullptr);
    }
}

/**
 * @brief run event-processing thread
 */
void
EventQueue::EventQueue::run()
{
    while (m_abort == false) {
        // get event
        Event* event = getEventFromQueue();
        if (event == nullptr) {
            // sleep if no event exist in the queue
            sleepThread(10000);
        }
        else {
            event->processEvent();
            if (m_deleteEventObj) {
                delete event;
            }
        }
    }
}

}  // namespace Hanami
