/**
 * @file       event.cpp
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

#include <hanami_common/threading/event.h>

#include <thread>

namespace Hanami
{

//==================================================================================================
// Event
//==================================================================================================
Event::~Event() {}

//==================================================================================================
// SleepEvent
//==================================================================================================

/**
 * @brief constructor
 * @param milliSeconds time in milli-seconds, which the event should sleep
 */
SleepEvent::SleepEvent(const uint64_t milliSeconds) { m_milliSeconds = milliSeconds; }

/**
 * @brief destructor
 */
SleepEvent::~SleepEvent() {}

/**
 * @brief process sleep-event
 * @return false, if invalid time was set of zero milliseconds was set, else true
 */
bool
SleepEvent::processEvent()
{
    if (m_milliSeconds == 0) {
        return false;
    }

    std::this_thread::sleep_for(std::chrono::milliseconds(m_milliSeconds));

    return true;
}

}  // namespace Hanami
