/**
 * @file    statemachine_test.cpp
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

#include "statemachine_test.h"

#include <hanami_common/statemachine.h>
#include <hanami_common/threading/event.h>

namespace Hanami
{

Statemachine_Test::Statemachine_Test() : Hanami::MemoryLeakTestHelpter("Statemachine_Test")
{
    create_delete_test();
}

/**
 * @brief create_delete_test
 */
void
Statemachine_Test::create_delete_test()
{
    REINIT_TEST();

    Statemachine* testMachine = new Statemachine();
    testMachine->createNewState(1);
    testMachine->createNewState(2);
    testMachine->addEventToState(1, new SleepEvent(10000));
    testMachine->addEventToState(2, new SleepEvent(10000));
    delete testMachine;

    CHECK_MEMORY();
}

}  // namespace Hanami
