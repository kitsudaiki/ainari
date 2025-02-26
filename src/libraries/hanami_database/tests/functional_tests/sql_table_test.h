/**
 * @file        sql_table_test.h
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

#ifndef SQLTABLE_TEST_H
#define SQLTABLE_TEST_H

#include <hanami_common/test_helper/compare_test_helper.h>

namespace Hanami
{

class SqlDatabase;
class TestTable;

class SqlTable_Test : public Hanami::CompareTestHelper
{
   public:
    SqlTable_Test();

   private:
    std::string m_filePath = "";
    TestTable* m_table = nullptr;
    SqlDatabase* m_db = nullptr;
    std::string m_name1 = "user0815";
    std::string m_name2 = "other";

    void deleteFile();
    void initTest();
    void initDatabase_test();

    void initTable_test();
    void create_test();
    void get_test();
    void getAll_test();
    void update_test();
    void delete_test();
    void getNumberOfRows_test();
};

}  // namespace Hanami

#endif  // SQLTABLE_TEST_H
