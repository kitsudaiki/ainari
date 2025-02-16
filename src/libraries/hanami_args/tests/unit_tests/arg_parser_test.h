/**
 * @file       arg_parser_test.h
 *
 *  @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *  @copyright  Apache License Version 2.0
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

#ifndef ARG_PARSER_TEST_H
#define ARG_PARSER_TEST_H

#include <assert.h>
#include <hanami_common/test_helper/compare_test_helper.h>

namespace Hanami
{
class ArgParser;

class ArgParser_Test : public Hanami::CompareTestHelper
{
   public:
    ArgParser_Test();

    void getArgument_test();
    void convertValue_test();

    void parse_test();

    void getNumberOfValues_test();
    void getStringValues_test();
    void getIntValues_test();
    void getFloatValues_test();
    void getBoolValues_test();

    void getStringValue_test();
    void getIntValue_test();
    void getFloatValue_test();
    void getBoolValue_test();

   private:
    void prepareTest(ArgParser& parser);
};

}  // namespace Hanami

#endif  // ARG_PARSER_TEST_H
