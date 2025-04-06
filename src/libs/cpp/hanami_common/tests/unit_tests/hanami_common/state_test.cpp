/**
 * @file    state_test.cpp
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

#include "state_test.h"

#include <state.h>

namespace Hanami
{

State_Test::State_Test() : Hanami::CompareTestHelper("State_Test")
{
    addTransition_test();
    next_test();
    setInitialChildState_test();
    addChildState_test();
}

/**
 * addTransition_test
 */
void
State_Test::addTransition_test()
{
    State sourceState(SOURCE_STATE);
    State nextState(NEXT_STATE);

    TEST_EQUAL(sourceState.addTransition(GO, &nextState), true);
    TEST_EQUAL(sourceState.addTransition(GOGO, &nextState), true);
    TEST_EQUAL(sourceState.addTransition(GO, &nextState), false);

    TEST_EQUAL(sourceState.nextStates.size(), 2);
}

/**
 * next_test
 */
void
State_Test::next_test()
{
    State sourceState(SOURCE_STATE);
    State nextState(NEXT_STATE);
    State* selctedState = nullptr;
    bool isNullptr = false;

    sourceState.addTransition(GO, &nextState);

    selctedState = sourceState.next(GO);
    isNullptr = selctedState == nullptr;
    TEST_EQUAL(isNullptr, false);
    TEST_EQUAL(selctedState->id, NEXT_STATE);

    selctedState = sourceState.next(FAIL);
    isNullptr = selctedState == nullptr;
    TEST_EQUAL(isNullptr, true);
}

/**
 * setInitialChildState_test
 */
void
State_Test::setInitialChildState_test()
{
    State sourceState(SOURCE_STATE);
    State initialState(INITIAL_STATE);

    sourceState.setInitialChildState(&initialState);
    TEST_EQUAL(sourceState.initialChild->id, INITIAL_STATE);
}

/**
 * addChildState_test
 */
void
State_Test::addChildState_test()
{
    State sourceState(SOURCE_STATE);
    State childState(CHILD_STATE);

    sourceState.addChildState(&childState);
    TEST_EQUAL(childState.parent->id, SOURCE_STATE);
}

}  // namespace Hanami
