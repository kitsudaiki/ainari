/**
 * @file    ini_item_test.cpp
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

#include "ini_item_test.h"

#include <hanami_ini/ini_item.h>

#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace Hanami
{

IniItem_Test::IniItem_Test() : Hanami::CompareTestHelper("IniItem_Test")
{
    parse_test();
    get_test();
    set_test();
    removeGroup_test();
    removeEntry_test();
    print_test();
}

/**
 * parse_test
 */
void
IniItem_Test::parse_test()
{
    IniItem object;
    ErrorContainer error;

    bool result = object.parse(getTestString(), error);

    TEST_EQUAL(result, true);
    if (result == false) {
        std::cout << "errorMessage: " << error.toString() << std::endl;
    }
}

/**
 * get_test
 */
void
IniItem_Test::get_test()
{
    IniItem object;
    ErrorContainer error;

    object.parse(getTestString(), error);
    json ret;
    TEST_EQUAL(object.get(ret, "DEFAULT", "x"), true);
    TEST_EQUAL(ret.dump(), "2");

    TEST_EQUAL(object.get(ret, "hmmm", "poi_poi"), true);
    TEST_EQUAL(ret.dump(), "1.3");

    TEST_EQUAL(object.get(ret, "hmmm", "bool_value"), true);
    TEST_EQUAL(ret, true);

    TEST_EQUAL(object.get(ret, "hmmm", "bool_value2"), true);
    TEST_EQUAL(ret, true);
}

/**
 * set_test
 */
void
IniItem_Test::set_test()
{
    IniItem object;
    ErrorContainer error;

    object.parse(getTestString(), error);

    TEST_EQUAL(object.set("hmmm2", "poi", "asdf"), true);
    TEST_EQUAL(object.set("hmmm2", "poi", "asdf"), false);

    TEST_EQUAL(object.set("hmmm", "poi_poi", "asdf", true), true);

    json ret;
    TEST_EQUAL(object.get(ret, "hmmm", "poi_poi"), true);
    TEST_EQUAL(ret, "asdf");
}

/**
 * removeGroup_test
 */
void
IniItem_Test::removeGroup_test()
{
    IniItem object;
    ErrorContainer error;

    object.parse(getTestString(), error);

    TEST_EQUAL(object.removeGroup("hmmm"), true);
    TEST_EQUAL(object.removeGroup("hmmm"), false);

    const std::string compare(
        "[DEFAULT]\n"
        "asdf = asdf.asdf\n"
        "id = 550e8400-e29b-11d4-a716-446655440000\n"
        "x = 2\n"
        "y = \n"
        "\n");

    TEST_EQUAL(object.toString(), compare);
}

/**
 * removeEntry_test
 */
void
IniItem_Test::removeEntry_test()
{
    IniItem object;
    ErrorContainer error;

    object.parse(getTestString(), error);

    TEST_EQUAL(object.removeEntry("DEFAULT", "x"), true);
    TEST_EQUAL(object.removeEntry("fail", "x"), false);

    const std::string compare(
        "[DEFAULT]\n"
        "asdf = asdf.asdf\n"
        "id = 550e8400-e29b-11d4-a716-446655440000\n"
        "y = \n"
        "\n"
        "[hmmm]\n"
        "bool_value = true\n"
        "bool_value2 = true\n"
        "poi_poi = 1.3\n"
        "\n");

    TEST_EQUAL(object.toString(), compare);
}

/**
 * @brief helper-function for remove characters
 */
bool
isSlash(char c)
{
    if (c == '\"') {
        return true;
    }
    else {
        return false;
    }
}

/**
 * print_test
 */
void
IniItem_Test::print_test()
{
    IniItem object;
    ErrorContainer error;

    object.parse(getTestString(), error);

    const std::string outputStringObjects = object.toString();

    const std::string compare(
        "[DEFAULT]\n"
        "asdf = asdf.asdf\n"
        "id = 550e8400-e29b-11d4-a716-446655440000\n"
        "x = 2\n"
        "y = \n"
        "\n"
        "[hmmm]\n"
        "bool_value = true\n"
        "bool_value2 = true\n"
        "poi_poi = 1.3\n"
        "\n");
    TEST_EQUAL(outputStringObjects, compare);

    // negative test
    const std::string badString(
        "[DEFAULT]\n"
        "asdf = asdf.asdf\n"
        "id = 550e8400-e29b-11d4-a716-446655440000\n"
        "x = 2\n"
        "y = 3\n"
        "\n"
        "(hmmm]\n"
        "bool_value = true\n"
        "bool_value2 = true\n"
        "poi_poi = 1.3\n"
        "\n");
    bool result = object.parse(badString, error);
    TEST_EQUAL(result, false);

    const std::string compareError
        = "+---------------------+------------------------------------------+\n"
          "| Error-Message Nr. 0 | ERROR while parsing ini-formated string  |\n"
          "|                     | parser-message: invalid character        |\n"
          "|                     | line-number: 7                           |\n"
          "|                     | position in line: 1                      |\n"
          "|                     | broken part in string: \"(\"               |\n"
          "+---------------------+------------------------------------------+\n";
    TEST_EQUAL(error.toString(), compareError);
}

/**
 * @brief get test-string
 */
const std::string
IniItem_Test::getTestString()
{
    const std::string testString(
        "[DEFAULT]\n"
        "asdf = asdf.asdf\n"
        "id = 550e8400-e29b-11d4-a716-446655440000\n"
        "x = 2\n"
        "y = \n"
        "\n\n"
        "[hmmm]\n"
        "# this is only a simple 0815 test-comment\n\n"
        "bool_value = true\n"
        "bool_value2 = True\n"
        "poi_poi = 1.3\n"
        "\n");
    return testString;
}

}  // namespace Hanami
