/**
 * @file    statemachine_test.h
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

#ifndef STATEMACHINE_TEST_H
#define STATEMACHINE_TEST_H

#include <hanami_common/test_helper/compare_test_helper.h>

namespace Hanami
{

class Statemachine_Test : public Hanami::CompareTestHelper
{
   public:
    Statemachine_Test();

   private:
    enum states {
        SOURCE_STATE = 1,
        TARGET_STATE = 2,
        CHILD_STATE = 3,
        NEXT_STATE = 4,
        GO = 5,
        GOGO = 6,
        FAIL = 7,
    };

    void createNewState_test();
    void setCurrentState_test();
    void addTransition_test();
    void goToNextState_test();
    void getCurrentStateId_test();
    void getCurrentStateName_test();
    void setInitialChildState_test();
    void addChildState_test();
    void isInState_test();
};

}  // namespace Hanami

#endif  // STATEMACHINE_TEST_H
