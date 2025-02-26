/**
 * @file    state_test.cpp
 *
 * @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright  Apache License Version 2.0
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

State_Test::State_Test() : Hanami::MemoryLeakTestHelpter("State_Test") { create_delete_test(); }

/**
 * @brief create_delete_test
 */
void
State_Test::create_delete_test()
{
    REINIT_TEST();

    State* testState = new State(42, "test-state");
    delete testState;

    CHECK_MEMORY();
}

}  // namespace Hanami
