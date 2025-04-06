/**
 * @file       statemachine.h
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

#ifndef HANAMI_STATEMACHINE_H
#define HANAMI_STATEMACHINE_H

#include <atomic>
#include <map>
#include <string>

namespace Hanami
{
struct State;
class Event;
class EventQueue;

class Statemachine
{
   public:
    Statemachine(EventQueue* eventQueue = nullptr);
    ~Statemachine();

    // init
    bool createNewState(const uint32_t stateId, const std::string& stateName = "");
    bool setCurrentState(const uint32_t stateId);
    bool addTransition(const uint32_t stateId, const uint32_t key, const uint32_t nextStateId);
    bool setInitialChildState(const uint32_t stateId, const uint32_t initialChildStateId);
    bool addChildState(const uint32_t stateId, const uint32_t childStateId);

    bool addEventToState(const uint32_t stateId, Event* event);

    // runtime
    bool goToNextState(const uint32_t nextStateId, const uint32_t requiredPreState = 0);
    uint32_t getCurrentStateId();
    const std::string getCurrentStateName();
    bool isInState(const uint32_t stateId);

   private:
    std::map<uint32_t, State*> m_allStates;
    State* m_currentState = nullptr;
    EventQueue* m_eventQueue = nullptr;
    std::atomic_flag m_state_lock = ATOMIC_FLAG_INIT;

    State* getState(const uint32_t stateId);
};

}  // namespace Hanami

#endif  // HANAMI_STATEMACHINE_H
