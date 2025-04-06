/**
 * @file    table_item_test.h
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

#ifndef TABLE_ITEM_TEST_H
#define TABLE_ITEM_TEST_H

#include <hanami_common/items/table_item.h>
#include <hanami_common/test_helper/compare_test_helper.h>

#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace Hanami
{

class TableItem_test : public Hanami::CompareTestHelper
{
   public:
    TableItem_test();

   private:
    void copy_contructor_test();
    void assignment_operator_test();
    void clearTable_test();
    void stealContent_test();

    // column
    void addColumn_Test();
    void renameColume_Test();
    void deleteColumn_Test();

    // row
    void addRow_Test();
    void getRow_Test();
    void deleteRow_Test();

    // cell
    void setCell_Test();
    void getCell_Test();
    void deleteCell_Test();

    // size
    void getNumberOfColums_Test();
    void getNumberOfRows_Test();

    // output
    void getInnerHeader_test();
    void toString_test();
    void toJsonString_test();

    TableItem getTestTableItem();
};

}  // namespace Hanami

#endif  // TABLE_ITEM_TEST_H
