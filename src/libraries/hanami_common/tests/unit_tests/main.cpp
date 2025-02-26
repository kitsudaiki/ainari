/**
 * @file    main.cpp
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

#include <hanami_common/buffer/bit_buffer_test.h>
#include <hanami_common/buffer/data_buffer_test.h>
#include <hanami_common/buffer/item_buffer_test.h>
#include <hanami_common/buffer/ring_buffer_test.h>
#include <hanami_common/buffer/stack_buffer_reserve_test.h>
#include <hanami_common/buffer/stack_buffer_test.h>
#include <hanami_common/files/binary_file_with_directIO_test.h>
#include <hanami_common/files/binary_file_without_directIO_test.h>
#include <hanami_common/files/text_file_test.h>
#include <hanami_common/functions/file_functions_test.h>
#include <hanami_common/functions/string_functions_test.h>
#include <hanami_common/functions/vector_functions_test.h>
#include <hanami_common/items/table_item_test.h>
#include <hanami_common/logger_test.h>
#include <hanami_common/progress_bar_test.h>
#include <hanami_common/state_test.h>
#include <hanami_common/statemachine_test.h>
#include <hanami_common/threading/thread_handler_test.h>

int
main()
{
    Hanami::BitBuffer_Test();
    Hanami::DataBuffer_Test();
    Hanami::ItemBuffer_Test();
    Hanami::RingBuffer_Test();
    Hanami::StackBufferReserve_Test();
    Hanami::StackBuffer_Test();

    Hanami::Stringfunctions_Test();
    Hanami::Vectorfunctions_Test();
    Hanami::Filefunctions_Test();

    Hanami::State_Test();
    Hanami::Statemachine_Test();
    Hanami::ProgressBar_Test();
    Hanami::Logger_Test();

    Hanami::ThreadHandler_Test();

    Hanami::TextFile_Test();
    Hanami::BinaryFile_withDirectIO_Test();
    Hanami::BinaryFile_withoutDirectIO_Test();

    Hanami::TableItem_test();
}
