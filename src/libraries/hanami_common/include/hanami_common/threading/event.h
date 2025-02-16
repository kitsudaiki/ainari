/**
 * @file       event.h
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

#ifndef EVENT_H
#define EVENT_H

#include <stdint.h>

namespace Hanami
{

//===================================================================
// Event
//===================================================================
class Event
{
   public:
    virtual ~Event();

    virtual bool processEvent() = 0;
};

//===================================================================
// SleepEvent
//===================================================================
class SleepEvent : public Event
{
   public:
    SleepEvent(const uint64_t milliSeconds);
    ~SleepEvent();

    bool processEvent();

   private:
    uint64_t m_milliSeconds = 0;
};

}  // namespace Hanami

#endif  // EVENT_H
