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

#include <hanami_common/buffer/data_buffer_test.h>
#include <hanami_common/buffer/item_buffer_test.h>
#include <hanami_common/buffer/ring_buffer_test.h>
#include <hanami_common/buffer/stack_buffer_reserve_test.h>
#include <hanami_common/buffer/stack_buffer_test.h>
#include <hanami_common/items/table_item_test.h>
#include <hanami_common/state_test.h>
#include <hanami_common/statemachine_test.h>
#include <hanami_common/threading/thread_test.h>

int
main()
{
    Hanami::DataBuffer_Test();
    Hanami::ItemBuffer_Test();
    Hanami::RingBuffer_Test();
    Hanami::StackBufferReserve_Test();
    Hanami::StackBuffer_Test();

    Hanami::State_Test();
    Hanami::Statemachine_Test();

    Hanami::TableItem_test();

    Hanami::Thread_Test();
}
