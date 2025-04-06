/**
 * @file    stack_buffer_reserve_test.cpp
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

#include "stack_buffer_reserve_test.h"

#include <hanami_common/buffer/stack_buffer_reserve.h>

namespace Hanami
{

StackBufferReserve_Test::StackBufferReserve_Test()
    : Hanami::CompareTestHelper("StackBufferReserve_Test")
{
    addBuffer_test();
    getNumberOfBuffers_test();
    getBuffer_test();
}

/**
 * @brief addStage_test
 */
void
StackBufferReserve_Test::addBuffer_test()
{
    // init
    StackBufferReserve stackBufferReserve;
    DataBuffer* newBuffer = new DataBuffer();

    // run test
    TEST_EQUAL(stackBufferReserve.addBuffer(nullptr), false);
    TEST_EQUAL(stackBufferReserve.addBuffer(newBuffer), true);
}

/**
 * @brief getNumberOfStages_test
 */
void
StackBufferReserve_Test::getNumberOfBuffers_test()
{
    // init
    uint32_t reserveSize = 10;
    StackBufferReserve stackBufferReserve(reserveSize);
    DataBuffer* newBuffer = new DataBuffer();

    // test normal add
    TEST_EQUAL(stackBufferReserve.getNumberOfBuffers(), 0);
    stackBufferReserve.addBuffer(newBuffer);
    TEST_EQUAL(stackBufferReserve.getNumberOfBuffers(), 1);

    // test max size
    for (uint32_t i = 0; i < reserveSize + 10; i++) {
        stackBufferReserve.addBuffer(new DataBuffer());
    }

    TEST_EQUAL(stackBufferReserve.getNumberOfBuffers(), reserveSize);
}

/**
 * @brief getStage_test
 */
void
StackBufferReserve_Test::getBuffer_test()
{
    // init
    StackBufferReserve stackBufferReserve;
    DataBuffer* newBuffer = new DataBuffer();
    DataBuffer* returnBuffer = nullptr;
    stackBufferReserve.addBuffer(newBuffer);

    // run test
    TEST_EQUAL(stackBufferReserve.getNumberOfBuffers(), 1);
    returnBuffer = stackBufferReserve.getBuffer();
    delete returnBuffer;
    TEST_EQUAL(stackBufferReserve.getNumberOfBuffers(), 0);
    returnBuffer = stackBufferReserve.getBuffer();
    delete returnBuffer;
    TEST_EQUAL(stackBufferReserve.getNumberOfBuffers(), 0);
    returnBuffer = stackBufferReserve.getBuffer();
    delete returnBuffer;
}

}  // namespace Hanami
