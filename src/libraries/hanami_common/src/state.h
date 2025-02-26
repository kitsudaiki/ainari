/**
 * @file       state.h
 *
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
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

#ifndef STATE_H
#define STATE_H

#include <hanami_common/threading/event.h>
#include <stdint.h>

#include <map>
#include <string>
#include <utility>
#include <vector>

namespace Hanami
{

struct State {
    std::map<uint32_t, State*> nextStates;
    std::string name = "";
    uint32_t id = 0;
    State* parent = nullptr;
    State* initialChild = nullptr;
    std::vector<Event*> events;

    /**
     * @brief constructor
     */
    State(const uint32_t id, const std::string& name = "")
    {
        this->id = id;
        this->name = name;
    }

    /**
     * @brief add a ne transition to another state
     *
     * @param key key-value which identify the transistion
     * @param nextState next state with belongs to the spezific key
     *
     * @return false if key already registerd, else true
     */
    bool addTransition(const uint32_t& key, State* nextState)
    {
        return nextStates.try_emplace(key, nextState).second;
    }

    /**
     * @brief get the next requested state
     *
     * @param key identifier-key for the next state
     *
     * @return pointer to the next state or the next state of the parent,
     *         returns null-pointer if the key is unknown
     */
    State* next(const uint32_t& key)
    {
        auto it = nextStates.find(key);
        if (it != nextStates.end()) {
            State* tempState = it->second;
            if (tempState->initialChild != nullptr) {
                tempState = tempState->initialChild;
            }
            return tempState;
        }

        return nullptr;
    }

    /**
     * @brief set the initial child state
     *
     * @param child initial child state
     */
    void setInitialChildState(State* child) { this->initialChild = child; }

    /**
     * @brief add child state
     *
     * @param child new child state
     */
    void addChildState(State* child) { child->parent = this; }

    /**
     * @brief add new event to the state
     *
     * @param event new event, which should be triggered, when enter the state
     */
    void addEvent(Event* event) { events.push_back(event); }

    /**
     * @brief process all events in this state
     */
    void processEvents()
    {
        for (uint64_t i = 0; i < events.size(); i++) {
            events.at(i)->processEvent();
        }
    }
};

}  // namespace Hanami

#endif  // STATE_H
