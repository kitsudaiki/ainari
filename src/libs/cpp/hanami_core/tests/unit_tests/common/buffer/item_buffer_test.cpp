/**
 * @file    item_buffer_test.cpp
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

#include "item_buffer_test.h"

#include <src/common/buffer/item_buffer.h>

namespace Hanami
{

struct TestStruct {
    uint8_t a = 0;
    uint8_t b = 0;
    uint64_t c = 0;
} __attribute__((packed));

ItemBuffer_Test::ItemBuffer_Test() : Hanami::CompareTestHelper("ItemBuffer_Test")
{
    initBuffer_test();
    deleteItem_test();
    deleteAll_test();
    addNewItem_test();
}

/**
 * initBuffer_test
 */
void
ItemBuffer_Test::initBuffer_test()
{
    ItemBuffer<TestStruct> testBuffer;

    // init buffer
    TEST_EQUAL(testBuffer.initBuffer(42000), true);
    TEST_EQUAL(testBuffer.metaData.itemSize, sizeof(TestStruct));
    TEST_EQUAL(testBuffer.metaData.itemCapacity, 42000);
    TEST_EQUAL(testBuffer.metaData.numberOfItems, 42000);

    // test override of an already initialized buffer
    TEST_EQUAL(testBuffer.initBuffer(43000), true);
    TEST_EQUAL(testBuffer.metaData.itemSize, sizeof(TestStruct));
    TEST_EQUAL(testBuffer.metaData.itemCapacity, 43000);
    TEST_EQUAL(testBuffer.metaData.numberOfItems, 43000);
}

/**
 * deleteItem_test
 */
void
ItemBuffer_Test::deleteItem_test()
{
    // prepare item-buffer
    ItemBuffer<TestStruct> testBuffer;

    // check failing delete in not initialized buffer
    TEST_EQUAL(testBuffer.deleteItem(42), false);

    // init test-buffer
    testBuffer.initBuffer(42000);

    // delete items from test-buffer
    TEST_EQUAL(testBuffer.deleteItem(42), true);
    TEST_EQUAL(testBuffer.deleteItem(43), true);
    TEST_EQUAL(testBuffer.deleteItem(44), true);
    TEST_EQUAL(testBuffer.metaData.numberOfItems, 41997);
    TEST_EQUAL(testBuffer.deleteItem(42), false);
}

/**
 * deleteAll_test
 */
void
ItemBuffer_Test::deleteAll_test()
{
    // prepare item-buffer
    ItemBuffer<TestStruct> testBuffer;
    testBuffer.initBuffer(42000);

    // delete all items and check buffer-values
    testBuffer.deleteAll();
    TEST_EQUAL(testBuffer.metaData.numberOfItems, 0);
    TEST_EQUAL(testBuffer.metaData.itemSize, sizeof(TestStruct));
    TEST_EQUAL(testBuffer.metaData.itemCapacity, 42000);
}

/**
 * addNewItem_test
 */
void
ItemBuffer_Test::addNewItem_test()
{
    // prepare item-buffer
    ItemBuffer<TestStruct> testBuffer;
    testBuffer.initBuffer(42000);
    testBuffer.deleteItem(42);
    testBuffer.deleteItem(43);
    testBuffer.deleteItem(44);

    // prepare test-struct
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.b = 42;
    testStruct.c = 1337;

    // add items and check buffer-values
    TEST_EQUAL(testBuffer.addNewItem(testStruct), 42);
    TEST_EQUAL(testBuffer.metaData.numberOfItems, 41998);
    TEST_EQUAL(testBuffer.metaData.itemSize, sizeof(TestStruct));
    TEST_EQUAL(testBuffer.metaData.itemCapacity, 42000);
    TEST_EQUAL(testBuffer.addNewItem(testStruct), 43);
    TEST_EQUAL(testBuffer.addNewItem(testStruct), 44);
    TEST_EQUAL(testBuffer.addNewItem(testStruct), undefinedPos);

    // check if delete and reuse still works, after the complete buffer was filled
    TEST_EQUAL(testBuffer.deleteItem(42), true);
    TEST_EQUAL(testBuffer.addNewItem(testStruct), 42);

    // check content of the added items
    TestStruct* structs = getItemData<TestStruct>(testBuffer);
    TEST_EQUAL(structs[42].c, 1337);
    TEST_EQUAL(structs[43].c, 1337);
    TEST_EQUAL(structs[44].c, 1337);
}

}  // namespace Hanami
