/**
 * @file    ring_buffer_test.cpp
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

#include "ring_buffer_test.h"

#include <hanami_common/buffer/ring_buffer.h>

namespace Hanami
{

RingBuffer_Test::RingBuffer_Test() : Hanami::MemoryLeakTestHelpter("RingBuffer_Test")
{
    create_delete_test();
}

/**
 * @brief create_delete_test
 */
void
RingBuffer_Test::create_delete_test()
{
    REINIT_TEST();

    RingBuffer* testBuffer = new RingBuffer();
    delete testBuffer;

    CHECK_MEMORY();
}

}  // namespace Hanami
