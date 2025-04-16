/**
 * @file    table_item_test.cpp
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

#include "table_item_test.h"

namespace Hanami
{

TableItem_test::TableItem_test() : Hanami::CompareTestHelper("TableItem_test")
{
    copy_contructor_test();
    assignment_operator_test();
    stealContent_test();

    // column
    addColumn_Test();
    renameColume_Test();
    deleteColumn_Test();

    // row
    addRow_Test();
    getRow_Test();
    deleteRow_Test();

    // cell
    setCell_Test();
    getCell_Test();
    deleteCell_Test();

    // size
    getNumberOfColums_Test();
    getNumberOfRows_Test();

    // output
    getInnerHeader_test();
    toString_test();
    toJsonString_test();
}

/**
 * @brief copy_contructor_test
 */
void
TableItem_test::copy_contructor_test()
{
    TableItem testItem = getTestTableItem();
    TableItem copyItem = testItem;

    TEST_EQUAL(copyItem.toString(), testItem.toString());
}

/**
 * @brief assignment_operator_test
 */
void
TableItem_test::assignment_operator_test()
{
    TableItem testItem = getTestTableItem();
    TableItem assignedItem;

    assignedItem = testItem;

    TEST_EQUAL(assignedItem.toString(), testItem.toString());
}

/**
 * @brief clearTable_test
 */
void
TableItem_test::clearTable_test()
{
    TableItem testItem = getTestTableItem();

    testItem.clearTable();

    TEST_EQUAL(testItem.getNumberOfRows(), 0);
    TEST_EQUAL(testItem.getNumberOfColums(), 0);
}

/**
 * @brief clearTable_test
 */
void
TableItem_test::stealContent_test()
{
    TableItem testItem = getTestTableItem();

    json data = testItem.stealContent();

    TEST_EQUAL(data.contains("header"), true);
    TEST_EQUAL(data.contains("body"), true);

    TEST_EQUAL(data["header"].size(), 2);
    TEST_EQUAL(data["body"].size(), 2);

    TEST_EQUAL(testItem.getNumberOfRows(), 2);
    TEST_EQUAL(testItem.getNumberOfColums(), 2);
}

/**
 * @brief addColumn_Test
 */
void
TableItem_test::addColumn_Test()
{
    TableItem testItem;

    TEST_EQUAL(testItem.addColumn("asdf", "ASDF"), true);
    TEST_EQUAL(testItem.addColumn("poi"), true);

    const std::string compare
        = "+------+-----+\n"
          "| ASDF | poi |\n"
          "+======+=====+\n";
    TEST_EQUAL(testItem.toString(), compare);
}

/**
 * @brief renameColume_Test
 */
void
TableItem_test::renameColume_Test()
{
    TableItem testItem;

    testItem.addColumn("asdf", "ASDF");

    TEST_EQUAL(testItem.renameColume("asdf", "XYZ"), true);
    TEST_EQUAL(testItem.renameColume("fail", "XYZ"), false);

    const std::string compare
        = "+-----+\n"
          "| XYZ |\n"
          "+=====+\n";
    TEST_EQUAL(testItem.toString(), compare);
}

/**
 * @brief deleteColumn_Test
 */
void
TableItem_test::deleteColumn_Test()
{
    TableItem testItem;

    testItem.addColumn("asdf", "ASDF");
    testItem.addColumn("poi");

    TEST_EQUAL(testItem.deleteColumn("asdf"), true);
    TEST_EQUAL(testItem.deleteColumn("fail"), false);

    const std::string compare
        = "+-----+\n"
          "| poi |\n"
          "+=====+\n";
    TEST_EQUAL(testItem.toString(), compare);
}

/**
 * @brief addRow_Test
 */
void
TableItem_test::addRow_Test()
{
    TableItem testItem;

    testItem.addColumn("asdf", "ASDF");
    testItem.addColumn("poipoipoi");

    TEST_EQUAL(testItem.addRowVec(std::vector<std::string>{"this is a test", "k"}), true);
    TEST_EQUAL(testItem.addRowVec(std::vector<std::string>{"asdf"}), false);

    json newRow = json::array();
    newRow.push_back("asdf");
    TEST_EQUAL(testItem.addRow(newRow), false);
    newRow.push_back(42);
    TEST_EQUAL(testItem.addRow(newRow), true);

    const std::string compare
        = "+----------------+-----------+\n"
          "| ASDF           | poipoipoi |\n"
          "+================+===========+\n"
          "| this is a test | k         |\n"
          "+----------------+-----------+\n"
          "| asdf           | 42        |\n"
          "+----------------+-----------+\n";

    TEST_EQUAL(testItem.toString(), compare);
}

/**
 * @brief getRow_Test
 */
void
TableItem_test::getRow_Test()
{
    TableItem testItem = getTestTableItem();
    json result;

    // additional multiline test
    testItem.addRowVec(std::vector<std::string>{"x\ny\nz", " "});
    testItem.addRowVec(std::vector<std::string>{"y", "abcdefghijklmnopqrst"});

    result = testItem.getRow(42);
    const bool isNullptr = result.is_null();
    TEST_EQUAL(isNullptr, true);

    result = testItem.getRow(1);
    const std::string compare = "[\"asdf\",\"qwert\"]";
    TEST_EQUAL(result.dump(), compare);
}

/**
 * @brief deleteRow_Test
 */
void
TableItem_test::deleteRow_Test()
{
    TableItem testItem = getTestTableItem();

    TEST_EQUAL(testItem.deleteRow(0), true);
    TEST_EQUAL(testItem.deleteRow(1), false);

    const std::string compare
        = "+------+-----------+\n"
          "| ASDF | poipoipoi |\n"
          "+======+===========+\n"
          "| asdf | qwert     |\n"
          "+------+-----------+\n";

    TEST_EQUAL(testItem.toString(), compare);
}

/**
 * @brief setCell_Test
 */
void
TableItem_test::setCell_Test()
{
    TableItem testItem = getTestTableItem();

    TEST_EQUAL(testItem.setCell(1, 1, "hmmm"), true);
    TEST_EQUAL(testItem.setCell(1, 10, "hmmm"), false);
    TEST_EQUAL(testItem.setCell(10, 1, "hmmm"), false);

    const std::string compare
        = "+----------------+-----------+\n"
          "| ASDF           | poipoipoi |\n"
          "+================+===========+\n"
          "| this is a test | k         |\n"
          "+----------------+-----------+\n"
          "| asdf           | hmmm      |\n"
          "+----------------+-----------+\n";

    TEST_EQUAL(testItem.toString(), compare);
}

/**
 * @brief getCell_Test
 */
void
TableItem_test::getCell_Test()
{
    TableItem testItem = getTestTableItem();

    TEST_EQUAL(testItem.getCell(0, 1), "asdf");
    TEST_EQUAL(testItem.getCell(10, 1), "");
    TEST_EQUAL(testItem.getCell(0, 10), "");
}

/**
 * @brief deleteCell_Test
 */
void
TableItem_test::deleteCell_Test()
{
    TableItem testItem = getTestTableItem();

    TEST_EQUAL(testItem.deleteCell(0, 1), true);
    TEST_EQUAL(testItem.deleteCell(10, 1), false);
    TEST_EQUAL(testItem.deleteCell(0, 10), false);

    const std::string compare
        = "+----------------+-----------+\n"
          "| ASDF           | poipoipoi |\n"
          "+================+===========+\n"
          "| this is a test | k         |\n"
          "+----------------+-----------+\n"
          "|                | qwert     |\n"
          "+----------------+-----------+\n";

    TEST_EQUAL(testItem.toString(), compare);
}

/**
 * @brief getNumberOfColums_Test
 */
void
TableItem_test::getNumberOfColums_Test()
{
    TableItem testItem = getTestTableItem();

    TEST_EQUAL(testItem.getNumberOfColums(), 2);
}

/**
 * @brief getNumberOfRows_Test
 */
void
TableItem_test::getNumberOfRows_Test()
{
    TableItem testItem = getTestTableItem();

    TEST_EQUAL(testItem.getNumberOfRows(), 2);
    testItem.deleteRow(0);
    TEST_EQUAL(testItem.getNumberOfRows(), 1);
}

/**
 * @brief TgetInnerHeader_test
 */
void
TableItem_test::getInnerHeader_test()
{
    TableItem testItem = getTestTableItem();

    json innerHader = testItem.getInnerHeader();
    TEST_EQUAL(innerHader.size(), 2);
    TEST_EQUAL(innerHader[0], "asdf");
    TEST_EQUAL(innerHader[1], "poipoipoi");
}

/**
 * @brief toString_test
 */
void
TableItem_test::toString_test()
{
    TableItem testItem = getTestTableItem();

    // additional multiline test
    testItem.addRowVec(std::vector<std::string>{"x\ny\nz", " "});
    testItem.addRowVec(std::vector<std::string>{"y", "abcdefghijklmnopqrst"});

    const std::string compare
        = "+-----------+-----------+\n"
          "| ASDF      | poipoipoi |\n"
          "+===========+===========+\n"
          "| this is a | k         |\n"
          "|  test     |           |\n"
          "+-----------+-----------+\n"
          "| asdf      | qwert     |\n"
          "+-----------+-----------+\n"
          "| x         |           |\n"
          "| y         |           |\n"
          "| z         |           |\n"
          "+-----------+-----------+\n"
          "| y         | abcdefghi |\n"
          "|           | jklmnopqr |\n"
          "|           | st        |\n"
          "+-----------+-----------+\n";

    // test with a maximum cell width of 9
    TEST_EQUAL(testItem.toString(9), compare);

    const std::string compareWithoutHeader
        = "+-----------+-----------+\n"
          "| this is a | k         |\n"
          "|  test     |           |\n"
          "+-----------+-----------+\n"
          "| asdf      | qwert     |\n"
          "+-----------+-----------+\n"
          "| x         |           |\n"
          "| y         |           |\n"
          "| z         |           |\n"
          "+-----------+-----------+\n"
          "| y         | abcdefghi |\n"
          "|           | jklmnopqr |\n"
          "|           | st        |\n"
          "+-----------+-----------+\n";

    // test with a maximum cell width of 9
    TEST_EQUAL(testItem.toString(9, true), compareWithoutHeader);
}

/**
 * @brief toJsonString_test
 */
void
TableItem_test::toJsonString_test()
{
    TableItem testItem = getTestTableItem();

    // additional multiline test
    testItem.addRowVec(std::vector<std::string>{"x\ny\nz", " "});
    testItem.addRowVec(std::vector<std::string>{"y", "abcdefghijklmnopqrst"});

    // check json-formated output
    const std::string compareJson
        = "{ header: [{\"inner\":\"asdf\",\"outer\":\"ASDF\"},"
          "{\"inner\":\"poipoipoi\",\"outer\":\"poipoipoi\"}], "
          "body: [[\"this is a test\",\"k\"],"
          "[\"asdf\",\"qwert\"],"
          "[\"x\\ny\\nz\",\" \"],"
          "[\"y\",\"abcdefghijklmnopqrst\"]]}";
    TEST_EQUAL(testItem.toJsonString(), compareJson);
}

/**
 * @brief TableItem_test::getTestTableItem
 * @return
 */
TableItem
TableItem_test::getTestTableItem()
{
    TableItem testItem;

    testItem.addColumn("asdf", "ASDF");
    testItem.addColumn("poipoipoi");

    testItem.addRowVec(std::vector<std::string>{"this is a test", "k"});
    testItem.addRowVec(std::vector<std::string>{"asdf", "qwert"});
    return testItem;
}

}  // namespace Hanami
