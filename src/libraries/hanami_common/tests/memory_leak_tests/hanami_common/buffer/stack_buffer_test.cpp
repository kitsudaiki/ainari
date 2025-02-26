/**
 * @file    stack_buffer_test.cpp
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

#include "stack_buffer_test.h"

#include <hanami_common/buffer/stack_buffer.h>

namespace Hanami
{

StackBuffer_Test::StackBuffer_Test() : Hanami::MemoryLeakTestHelpter("StackBuffer_Test")
{
    create_delete_test();
}

/**
 * @brief create_delete_test
 */
void
StackBuffer_Test::create_delete_test()
{
    // dummy-buffer is nessecary to init the internal static stack-buffer-reserve
    // which is not deleted with the stack-buffer and would break the test
    StackBuffer dummyBuffer;

    REINIT_TEST();

    StackBuffer* testBuffer = new StackBuffer();
    delete testBuffer;

    CHECK_MEMORY();
}

}  // namespace Hanami
