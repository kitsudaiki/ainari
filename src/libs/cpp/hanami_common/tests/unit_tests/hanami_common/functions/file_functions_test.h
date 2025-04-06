/**
 * @file      file_functions_test.h
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

#ifndef FILE_functions_TEST_H
#define FILE_functions_TEST_H

#include <hanami_common/test_helper/compare_test_helper.h>

namespace Hanami
{

class Filefunctions_Test : public Hanami::CompareTestHelper
{
   public:
    Filefunctions_Test();

   private:
    void listFiles_test();
    void renameFileOrDir_test();
    void copyPath_test();
    void createDirectory_test();
    void deleteFileOrDir_test();
};

}  // namespace Hanami

#endif  // FILE_functions_TEST_H
