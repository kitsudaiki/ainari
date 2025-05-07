/**
 * @file       statemachine.cpp
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

#include <hanami_common/statemachine.h>
#include <hanami_common/threading/event_queue.h>
#include <state.h>

namespace Hanami
{

/**
 * @brief constructor
 */
Statemachine::Statemachine(EventQueue* eventQueue) { m_eventQueue = eventQueue; }

/**
 * @brief destructor
 */
Statemachine::~Statemachine()
{
    // delete all states
    for (auto& [name, state] : m_allStates) {
        State* tempObj = state;
        for (Event* event : tempObj->events) {
            delete event;
        }
        delete tempObj;
        state = nullptr;
    }

    // clear map
    m_allStates.clear();
}

/**
 * @brief add a new state to the state-machine
 *
 * @param stateId id of the new state
 * @param stateName name of the new state
 *
 * @return false if state-id already exist, else true
 */
bool
Statemachine::createNewState(const uint32_t stateId, const std::string& stateName)
{
    while (m_state_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    // add new state
    State* newState = new State(stateId, stateName);
    auto ret = m_allStates.try_emplace(stateId, newState);
    if (ret.second == false) {
        delete newState;
        m_state_lock.clear(std::memory_order_release);
        return false;
    }

    // first created state is set as current stat to init the statemachine
    if (m_currentState == nullptr) {
        m_currentState = newState;
    }
    m_state_lock.clear(std::memory_order_release);

    return true;
}

/**
 * @brief set the current state of the statemachine to a specific state
 *
 * @param stateId id of the new state
 *
 * @return false, if state doesn't exist, else true
 */
bool
Statemachine::setCurrentState(const uint32_t stateId)
{
    // check if state already exist
    State* state = getState(stateId);
    if (state == nullptr) {
        return false;
    }

    m_currentState = state;

    return true;
}

/**
 * @brief add a ne transition to another state
 *
 * @param stateId source-state of the transition
 * @param key key-value which identify the transistion
 * @param nextStateId next state with belongs to the spezific key
 *
 * @return false if key already registerd or state or nextState doesn't exist, else true
 */
bool
Statemachine::addTransition(const uint32_t stateId, const uint32_t key, const uint32_t nextStateId)
{
    State* sourceState = getState(stateId);
    State* nextState = getState(nextStateId);

    if (sourceState == nullptr || nextState == nullptr) {
        return false;
    }

    // add transition
    const bool success = sourceState->addTransition(key, nextState);

    return success;
}

/**
 * @brief set initial child state
 *
 * @param stateId source-state of the transition
 * @param initialChildStateId id of the initial child-state
 *
 * @return false, if id doesn't exist, else true
 */
bool
Statemachine::setInitialChildState(const uint32_t stateId, const uint32_t initialChildStateId)
{
    State* sourceState = getState(stateId);
    State* initialChildState = getState(initialChildStateId);

    if (sourceState == nullptr || initialChildState == nullptr) {
        return false;
    }

    sourceState->setInitialChildState(initialChildState);

    return true;
}

/**
 * @brief add one state as child state for another one
 *
 * @param stateId source-state of the transition
 * @param childStateId id of the child-state
 *
 * @return false, if id doesn't exist, else true
 */
bool
Statemachine::addChildState(const uint32_t stateId, const uint32_t childStateId)
{
    State* sourceState = getState(stateId);
    State* childState = getState(childStateId);

    if (sourceState == nullptr || childState == nullptr) {
        return false;
    }

    sourceState->addChildState(childState);

    return true;
}

/**
 * @brief add a new event to a specific state, which should be triggered,
 *        when ever the state is entered
 *
 * @param stateId source-state of the transition
 * @param event event to trigger, when the state fill be entered
 *
 * @return false, if stateId doesn't exist or event is nullptr, else true
 */
bool
Statemachine::addEventToState(const uint32_t stateId, Event* event)
{
    State* sourceState = getState(stateId);

    if (sourceState == nullptr || event == nullptr) {
        return false;
    }

    sourceState->addEvent(event);

    return true;
}

/**
 * @brief got to the next state, if possible
 *
 * @param nextStateId the identifier of the next state of the statemachine
 * @param requiredPreState
 *
 * @return true, if there was the next requested state
 */
bool
Statemachine::goToNextState(const uint32_t nextStateId, const uint32_t requiredPreState)
{
    bool result = false;
    while (m_state_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    if (requiredPreState == 0 || requiredPreState == m_currentState->id) {
        State* state = m_currentState;
        while (state != nullptr) {
            State* nextState = state->next(nextStateId);
            state = state->parent;
            if (nextState != nullptr) {
                m_currentState = nextState;

                // add event of state to event-queue, if one was defined
                if (m_eventQueue != nullptr) {
                    for (uint64_t i = 0; i < m_currentState->events.size(); i++) {
                        m_eventQueue->addEventToQueue(m_currentState->events.at(i));
                    }
                }

                result = true;
                break;
            }
        }
    }

    m_state_lock.clear(std::memory_order_release);

    // process all events after enter the new state. This has to be done here at the end,
    // to release the spin-lock first. It is possible, that the event-processint take some time
    // and it would be really bad, when the spin-lock runs the whole time and block even requests
    // for the current state for the entire time.
    if (result == true && m_eventQueue == nullptr) {
        m_currentState->processEvents();
    }

    return result;
}

/**
 * @brief getter for the current machine-state-id
 *
 * @return the state-id of the statemachine
 */
uint32_t
Statemachine::getCurrentStateId()
{
    uint32_t result = 0;

    while (m_state_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    if (m_currentState != nullptr) {
        result = m_currentState->id;
    }

    m_state_lock.clear(std::memory_order_release);

    return result;
}

/**
 * @brief getter for the current machine-state-name
 *
 * @return the state-name of the statemachine
 */
const std::string
Statemachine::getCurrentStateName()
{
    std::string result = "";
    while (m_state_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    if (m_currentState != nullptr) {
        result = m_currentState->name;
    }

    m_state_lock.clear(std::memory_order_release);

    return result;
}

/**
 * @brief check if in statemachine is in a specific state
 *
 * @param stateId id of the requested state
 *
 * @return true, if in requested state or in a child-state of the requested state, else false
 */
bool
Statemachine::isInState(const uint32_t stateId)
{
    bool result = false;
    while (m_state_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    State* state = m_currentState;
    while (state != nullptr) {
        if (state->id == stateId) {
            result = true;
        }
        state = state->parent;
    }

    m_state_lock.clear(std::memory_order_release);

    return result;
}

/**
 * @brief get state by id
 *
 * @param stateId id of the state
 *
 * @return nullptr, if state-id was not found, else pointer to the state
 */
State*
Statemachine::getState(const uint32_t stateId)
{
    State* result = nullptr;
    while (m_state_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    // check and get source-state
    auto it = m_allStates.find(stateId);
    if (it != m_allStates.end()) {
        result = it->second;
    }

    m_state_lock.clear(std::memory_order_release);

    return result;
}

}  // namespace Hanami
