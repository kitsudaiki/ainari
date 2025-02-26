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

StackBuffer_Test::StackBuffer_Test() : Hanami::CompareTestHelper("StackBuffer_Test")
{
    constructor_test();
    extendBuffer_StackBuffer_test();
    addData_StackBuffer_test();
    addObject_StackBuffer_test();
    getFirstElement_StackBuffer_test();
    removeFirst_StackBuffer_test();
    reset_StackBuffer_test();
}

/**
 * @brief constructor_test
 */
void
StackBuffer_Test::constructor_test()
{
    // init
    bool isNullptr = false;

    // test constructor
    StackBuffer stackBuffer(10, 20);
    TEST_EQUAL(stackBuffer.preOffset, 10);
    TEST_EQUAL(stackBuffer.postOffset, 20);
    TEST_EQUAL(stackBuffer.effectiveBlockSize, STACK_BUFFER_BLOCK_SIZE - 30);
    TEST_EQUAL(stackBuffer.blockSize, STACK_BUFFER_BLOCK_SIZE);

    // test if m_stackBufferReserve is correctly set after first buffer-creation
    isNullptr = StackBufferReserve::getInstance() == nullptr;
    TEST_EQUAL(isNullptr, false);
}

/**
 * @brief extendBuffer_StackBuffer_test
 */
void
StackBuffer_Test::extendBuffer_StackBuffer_test()
{
    // init
    StackBuffer stackBuffer;

    // run test
    TEST_EQUAL(stackBuffer.blocks.size(), 0);
    extendBuffer_StackBuffer(stackBuffer);
    TEST_EQUAL(stackBuffer.blocks.size(), 1);
}

/**
 * @brief addData_StackBuffer_test
 */
void
StackBuffer_Test::addData_StackBuffer_test()
{
    // init
    StackBuffer stackBuffer;
    DataBuffer buffer(STACK_BUFFER_BLOCK_SIZE / 4096);

    // run test
    addData_StackBuffer(stackBuffer, buffer.data, 1000);
    TEST_EQUAL(stackBuffer.blocks.size(), 1);
    addData_StackBuffer(stackBuffer, buffer.data, 1000);
    TEST_EQUAL(stackBuffer.blocks.size(), 1);
    addData_StackBuffer(stackBuffer, buffer.data, buffer.totalBufferSize);
    TEST_EQUAL(stackBuffer.blocks.size(), 2);
}

/**
 * @brief addObject_StackBuffer_test
 */
void
StackBuffer_Test::addObject_StackBuffer_test()
{
    // init
    StackBuffer stackBuffer;
    uint64_t testValue = 42;

    // run test
    addObject_StackBuffer(stackBuffer, &testValue);
    TEST_EQUAL(stackBuffer.blocks.at(0)->usedBufferSize, sizeof(testValue));
}

/**
 * @brief getFirstElement_StackBuffer_test
 */
void
StackBuffer_Test::getFirstElement_StackBuffer_test()
{
    // init
    StackBuffer stackBuffer;
    DataBuffer buffer(STACK_BUFFER_BLOCK_SIZE / 4096);
    bool isNullptr = false;
    DataBuffer* result = nullptr;

    TEST_EQUAL(stackBuffer.blocks.size(), 0);
    result = getFirstElement_StackBuffer(stackBuffer);
    isNullptr = result == nullptr;
    TEST_EQUAL(isNullptr, true);
    TEST_EQUAL(stackBuffer.blocks.size(), 0);

    // prepare test-buffer
    uint64_t testValue = 42;
    addObject_DataBuffer(buffer, &testValue);
    addData_StackBuffer(stackBuffer, buffer.data, buffer.totalBufferSize);

    TEST_EQUAL(stackBuffer.blocks.size(), 1);
    result = getFirstElement_StackBuffer(stackBuffer);
    isNullptr = result == nullptr;
    TEST_EQUAL(isNullptr, false);
    TEST_EQUAL(stackBuffer.blocks.size(), 1);

    uint64_t ret = *static_cast<uint64_t*>(result->data);
    TEST_EQUAL(ret, testValue);
}

/**
 * @brief removeFirst_StackBuffer_test
 */
void
StackBuffer_Test::removeFirst_StackBuffer_test()
{
    // init
    StackBuffer stackBuffer;
    DataBuffer buffer(STACK_BUFFER_BLOCK_SIZE / 4096);

    // prepare test-buffer
    addData_StackBuffer(stackBuffer, buffer.data, buffer.totalBufferSize);
    addData_StackBuffer(stackBuffer, buffer.data, buffer.totalBufferSize);

    // run test
    TEST_EQUAL(stackBuffer.blocks.size(), 2);
    TEST_EQUAL(removeFirst_StackBuffer(stackBuffer), true);
    TEST_EQUAL(stackBuffer.blocks.size(), 1);
    TEST_EQUAL(removeFirst_StackBuffer(stackBuffer), true);
    TEST_EQUAL(stackBuffer.blocks.size(), 0);
    TEST_EQUAL(removeFirst_StackBuffer(stackBuffer), false);
}

/**
 * @brief reset_StackBuffer_test
 */
void
StackBuffer_Test::reset_StackBuffer_test()
{
    // init
    StackBuffer stackBuffer;
    DataBuffer buffer(STACK_BUFFER_BLOCK_SIZE / 4096);

    // prepare test-buffer
    addData_StackBuffer(stackBuffer, buffer.data, buffer.totalBufferSize);
    addData_StackBuffer(stackBuffer, buffer.data, buffer.totalBufferSize);

    // run test
    reset_StackBuffer(stackBuffer);

    // check result
    TEST_EQUAL(stackBuffer.blocks.size(), 0);
    bool isNullptr = stackBuffer.localReserve == nullptr;
    TEST_EQUAL(isNullptr, false);
}

}  // namespace Hanami
