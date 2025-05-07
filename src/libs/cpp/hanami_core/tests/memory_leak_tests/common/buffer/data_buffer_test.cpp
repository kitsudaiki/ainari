/**
 * @file    data_buffer_test.cpp
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
#include "data_buffer_test.h"

#include <src/common/buffer/data_buffer.h>

namespace Hanami
{

DataBuffer_Test::DataBuffer_Test() : Hanami::MemoryLeakTestHelpter("DataBuffer_Test")
{
    create_delete_test();
    fill_reset_test();
}

/**
 * @brief create_delete_test
 */
void
DataBuffer_Test::create_delete_test()
{
    REINIT_TEST();

    DataBuffer* testBuffer = new DataBuffer(10);
    delete testBuffer;

    CHECK_MEMORY();
}

/**
 * @brief fill_reset_test
 */
void
DataBuffer_Test::fill_reset_test()
{
    DataBuffer* testBuffer = new DataBuffer(10);

    REINIT_TEST();

    allocateBlocks_DataBuffer(*testBuffer, 42);
    reset_DataBuffer(*testBuffer, 10);

    CHECK_MEMORY();

    delete testBuffer;
}

}  // namespace Hanami
