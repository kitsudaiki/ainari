/**
 * @file    sqlite_test.cpp
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

#include "sqlite_test.h"

#include <hanami_common/items/table_item.h>
#include <hanami_sqlite/sqlite.h>

namespace Hanami
{

Sqlite_Test::Sqlite_Test() : Hanami::CompareTestHelper("Sqlite_Test")
{
    initTest();
    initDB_test();
    execSqlCommand_test();
    closeDB_test();
    closeTest();
}

/**
 * @brief initTest
 */
void
Sqlite_Test::initTest()
{
    m_filePath = "/tmp/testdb.db";
    deleteFile();
}

/**
 * @brief initDB_test
 */
void
Sqlite_Test::initDB_test()
{
    Sqlite testDB;

    ErrorContainer error;

    TEST_EQUAL(testDB.initDB(m_filePath, error), true);

    deleteFile();
}

/**
 * @brief execSqlCommand_test
 */
void
Sqlite_Test::execSqlCommand_test()
{
    Sqlite testDB;
    ErrorContainer error;
    testDB.initDB(m_filePath, error);

    Hanami::TableItem resultItem;

    //-----------------------------------------------------------------
    // CREATE TABLE
    //-----------------------------------------------------------------
    std::string sql
        = "CREATE TABLE COMPANY("
          "ID INT PRIMARY KEY     NOT NULL,"
          "NAME           TEXT    NOT NULL,"
          "AGE            INT     NOT NULL,"
          "ADDRESS        CHAR(50),"
          "SALARY         REAL );";

    TEST_EQUAL(testDB.execSqlCommand(nullptr, sql, error), true);

    //-----------------------------------------------------------------
    // INSERT
    //-----------------------------------------------------------------
    sql
        = "INSERT INTO COMPANY (ID,NAME,AGE,ADDRESS,SALARY) "
          "VALUES (1, 'Paul', 32, '{\"country\": \"California\"}', 20000.00 ); "
          "INSERT INTO COMPANY (ID,NAME,AGE,ADDRESS,SALARY) "
          "VALUES (2, 'Allen', 25, '{\"country\": \"Texas\"}', 15000.00 ); "
          "INSERT INTO COMPANY (ID,NAME,AGE,ADDRESS,SALARY)"
          "VALUES (3, 'Teddy', 23, '{\"country\": \"Norway\"}', 20000.00 );"
          "INSERT INTO COMPANY (ID,NAME,AGE,ADDRESS,SALARY)"
          "VALUES (4, 'Mark', 25, '{\"country\": \"Rich-Mond\"}', 65000.00 );";

    TEST_EQUAL(testDB.execSqlCommand(nullptr, sql, error), true);

    //-----------------------------------------------------------------
    // SELECT
    //-----------------------------------------------------------------
    sql = "SELECT * from COMPANY";

    resultItem.clearTable();
    TEST_EQUAL(testDB.execSqlCommand(&resultItem, sql, error), true);

    std::string compare
        = "+----+-------+-----+--------------------------+---------+\n"
          "| ID | NAME  | AGE | ADDRESS                  | SALARY  |\n"
          "+====+=======+=====+==========================+=========+\n"
          "| 1  | Paul  | 32  | {\"country\":\"California\"} | 20000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 2  | Allen | 25  | {\"country\":\"Texas\"}      | 15000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 3  | Teddy | 23  | {\"country\":\"Norway\"}     | 20000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 4  | Mark  | 25  | {\"country\":\"Rich-Mond\"}  | 65000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n";
    TEST_EQUAL(resultItem.toString(), compare);

    //-----------------------------------------------------------------
    // UPDATE
    //-----------------------------------------------------------------
    sql
        = "UPDATE COMPANY set SALARY = 25000.00 where ID=1; "
          "SELECT * from COMPANY";

    resultItem.clearTable();
    TEST_EQUAL(testDB.execSqlCommand(&resultItem, sql, error), true);

    compare
        = "+----+-------+-----+--------------------------+---------+\n"
          "| ID | NAME  | AGE | ADDRESS                  | SALARY  |\n"
          "+====+=======+=====+==========================+=========+\n"
          "| 1  | Paul  | 32  | {\"country\":\"California\"} | 25000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 2  | Allen | 25  | {\"country\":\"Texas\"}      | 15000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 3  | Teddy | 23  | {\"country\":\"Norway\"}     | 20000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 4  | Mark  | 25  | {\"country\":\"Rich-Mond\"}  | 65000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n";
    TEST_EQUAL(resultItem.toString(), compare);

    //-----------------------------------------------------------------
    // DELETE
    //-----------------------------------------------------------------
    sql
        = "DELETE from COMPANY where ID=2; "
          "SELECT * from COMPANY";

    resultItem.clearTable();
    TEST_EQUAL(testDB.execSqlCommand(&resultItem, sql, error), true);

    compare
        = "+----+-------+-----+--------------------------+---------+\n"
          "| ID | NAME  | AGE | ADDRESS                  | SALARY  |\n"
          "+====+=======+=====+==========================+=========+\n"
          "| 1  | Paul  | 32  | {\"country\":\"California\"} | 25000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 3  | Teddy | 23  | {\"country\":\"Norway\"}     | 20000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n"
          "| 4  | Mark  | 25  | {\"country\":\"Rich-Mond\"}  | 65000.0 |\n"
          "+----+-------+-----+--------------------------+---------+\n";
    TEST_EQUAL(resultItem.toString(), compare);

    testDB.closeDB();

    deleteFile();
}

/**
 * @brief closeDB_test
 */
void
Sqlite_Test::closeDB_test()
{
    Sqlite testDB;

    TEST_EQUAL(testDB.closeDB(), false);

    ErrorContainer error;
    testDB.initDB(m_filePath, error);

    TEST_EQUAL(testDB.closeDB(), true);
    TEST_EQUAL(testDB.closeDB(), false);

    deleteFile();
}

/**
 * @brief closeTest
 */
void
Sqlite_Test::closeTest()
{
    deleteFile();
}

/**
 * common usage to delete test-file
 */
void
Sqlite_Test::deleteFile()
{
    std::filesystem::path rootPathObj(m_filePath);
    if (std::filesystem::exists(rootPathObj)) {
        std::filesystem::remove(rootPathObj);
    }
}

}  // namespace Hanami
