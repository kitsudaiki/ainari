/**
 * @file       bit_buffer_test.cpp
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

#include "bit_buffer_test.h"

#include <hanami_common/buffer/bit_buffer.h>

namespace Hanami
{

BitBuffer_Test::BitBuffer_Test() : Hanami::CompareTestHelper("BitBuffer_Test")
{
    set_get_test();
    complete_test();
}

void
BitBuffer_Test::set_get_test()
{
    BitBuffer buffer(10);
    buffer.set(1, true);

    TEST_EQUAL(buffer.get(100), false);
    TEST_EQUAL(buffer.get(0), false);
    TEST_EQUAL(buffer.get(1), true);
    buffer.set(1, false);
    TEST_EQUAL(buffer.get(1), false);
}

void
BitBuffer_Test::complete_test()
{
    BitBuffer buffer(10);

    TEST_EQUAL(buffer.isComplete(), false);
    buffer.set(0, true);
    TEST_EQUAL(buffer.isComplete(), false);
    buffer.set(1, true);
    buffer.set(2, true);
    buffer.set(3, true);
    buffer.set(4, true);
    buffer.set(5, true);
    buffer.set(6, true);
    buffer.set(7, true);
    buffer.set(8, true);
    buffer.set(9, true);
    TEST_EQUAL(buffer.isComplete(), true);
}

}  // namespace Hanami
