/**
 * @file       arg_parser_test.cpp
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

#include "arg_parser_test.h"

#include <hanami_args/arg_parser.h>

namespace Hanami
{

ArgParser_Test::ArgParser_Test() : Hanami::CompareTestHelper("ArgParser_Test")
{
    getArgument_test();
    convertValue_test();

    parse_test();

    getNumberOfValues_test();
    getStringValues_test();
    getIntValues_test();
    getFloatValues_test();
    getBoolValues_test();

    getStringValue_test();
    getIntValue_test();
    getFloatValue_test();
    getBoolValue_test();
}

/**
 * @brief getArgument_test
 */
void
ArgParser_Test::getArgument_test()
{
    ArgParser parser;
    bool isNullptr = false;
    ErrorContainer error;

    parser.registerInteger("asdf", 'a').setHelpText("this is an example").setRequired();

    isNullptr = parser.getArgument("xyz") == nullptr;
    TEST_EQUAL(isNullptr, true);

    isNullptr = parser.getArgument("--asdf") == nullptr;
    TEST_EQUAL(isNullptr, false);

    isNullptr = parser.getArgument("-a") == nullptr;
    TEST_EQUAL(isNullptr, false);

    ArgParser::ArgDef* ret = parser.getArgument("-a");
    TEST_EQUAL(ret->type, ArgParser::ArgDef::INT_TYPE);
    TEST_EQUAL(ret->helpText, std::string("this is an example"));
    TEST_EQUAL(ret->isRequired, true);
    TEST_EQUAL(ret->withoutFlag, false);
    TEST_EQUAL(ret->longIdentifier, "--asdf");
    TEST_EQUAL(ret->shortIdentifier, "-a");
}

/**
 * @brief convertValue_test
 */
void
ArgParser_Test::convertValue_test()
{
    ArgParser parser;
    json result;
    bool isNullptr = false;
    ErrorContainer error;

    // string-type
    // check if result is nullptr
    isNullptr = parser.convertValue("asdf", ArgParser::ArgDef::STRING_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("1", ArgParser::ArgDef::STRING_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);

    // check result value
    result = parser.convertValue("asdf", ArgParser::ArgDef::STRING_TYPE);
    TEST_EQUAL(result.is_string(), true);
    TEST_EQUAL(result, "asdf");

    // int-type
    // check if result is nullptr
    isNullptr = parser.convertValue("1", ArgParser::ArgDef::INT_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("asdf", ArgParser::ArgDef::INT_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, true);

    // check result value
    result = parser.convertValue("42", ArgParser::ArgDef::INT_TYPE);
    TEST_EQUAL(result.is_number_integer(), true);
    TEST_EQUAL(result, 42);

    // float-type
    // check if result is nullptr
    isNullptr = parser.convertValue("1.0", ArgParser::ArgDef::FLOAT_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("1", ArgParser::ArgDef::FLOAT_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("asdf", ArgParser::ArgDef::FLOAT_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, true);

    // check result value
    result = parser.convertValue("42.25", ArgParser::ArgDef::FLOAT_TYPE);
    TEST_EQUAL(result.is_number_float(), true);
    TEST_EQUAL(result, 42.25);

    // bool-type
    // check if result is nullptr
    isNullptr = parser.convertValue("1", ArgParser::ArgDef::BOOL_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("true", ArgParser::ArgDef::BOOL_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("True", ArgParser::ArgDef::BOOL_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("0", ArgParser::ArgDef::BOOL_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("false", ArgParser::ArgDef::BOOL_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);
    isNullptr = parser.convertValue("False", ArgParser::ArgDef::BOOL_TYPE) == nullptr;
    TEST_EQUAL(isNullptr, false);

    // check result value
    result = parser.convertValue("true", ArgParser::ArgDef::BOOL_TYPE);
    TEST_EQUAL(result.is_boolean(), true);
    TEST_EQUAL(result, true);
}

/**
 * @brief parse_test
 */
void
ArgParser_Test::parse_test()
{
    ArgParser parser;
    ErrorContainer error;

    int argc = 15;
    const char* argv[15];
    argv[0] = "-";
    argv[1] = "--test";
    argv[2] = "test1";
    argv[3] = "--test";
    argv[4] = "test2";
    argv[5] = "--integer";
    argv[6] = "1337";
    argv[7] = "-f";
    argv[8] = "123.5";
    argv[9] = "-b";
    argv[10] = "true";
    argv[11] = "poi";
    argv[12] = "42";
    argv[13] = "42.25";
    argv[14] = "false";

    parser.registerString("test");
    parser.registerInteger("integer", 'i');
    parser.registerFloat("float", 'f');
    parser.registerBoolean("bool", 'b');
    parser.registerString("first_arg").setHelpText("first argument").setRequired().setWithoutFlag();
    parser.registerInteger("secondArg")
        .setHelpText("second argument")
        .setRequired()
        .setWithoutFlag();
    parser.registerFloat("thirdArg").setHelpText("third argument").setRequired().setWithoutFlag();
    parser.registerBoolean("lastArg").setHelpText("last argument").setRequired().setWithoutFlag();

    TEST_EQUAL(parser.parse(argc, argv, error), true);

    // negative test: set argument `bool` to a non-bool value
    argv[6] = "asdf";
    TEST_EQUAL(parser.parse(argc, argv, error), false);
    argv[6] = "true";

    // negative test: set a value without flag to a false type
    argv[12] = "asdf";
    TEST_EQUAL(parser.parse(argc, argv, error), false);
    argv[12] = "42";

    // negative test: register a required value, which is not given in the arguments
    parser.registerBoolean("fail").setHelpText("this is a boolean").setRequired();
    TEST_EQUAL(parser.parse(argc, argv, error), false);
}

/**
 * @brief getNumberOfValues_test
 */
void
ArgParser_Test::getNumberOfValues_test()
{
    ArgParser parser;
    prepareTest(parser);

    TEST_EQUAL(parser.getNumberOfValues("test"), 2);
    TEST_EQUAL(parser.getNumberOfValues("bool"), 1);
    TEST_EQUAL(parser.getNumberOfValues("first_arg"), 1);
}

/**
 * @brief getStringValues_test
 */
void
ArgParser_Test::getStringValues_test()
{
    ArgParser parser;
    prepareTest(parser);

    const std::vector<std::string> ret = parser.getStringValues("test");
    TEST_EQUAL(ret.size(), 2);
    TEST_EQUAL(ret.at(0), "test1");
    TEST_EQUAL(ret.at(1), "test2");

    const std::vector<std::string> ret2 = parser.getStringValues("first_arg");
    TEST_EQUAL(ret2.size(), 1);
    TEST_EQUAL(ret2.at(0), "poi");
}

/**
 * @brief getIntValues_test
 */
void
ArgParser_Test::getIntValues_test()
{
    ArgParser parser;
    prepareTest(parser);

    const std::vector<long> ret = parser.getIntValues("integer");
    TEST_EQUAL(ret.size(), 1);
    TEST_EQUAL(ret.at(0), 1337);

    const std::vector<long> ret2 = parser.getIntValues("secondArg");
    TEST_EQUAL(ret2.size(), 1);
    TEST_EQUAL(ret2.at(0), 42);
}

/**
 * @brief getFloatValues_test
 */
void
ArgParser_Test::getFloatValues_test()
{
    ArgParser parser;
    prepareTest(parser);

    const std::vector<double> ret = parser.getFloatValues("f");
    TEST_EQUAL(ret.size(), 1);
    TEST_EQUAL(ret.at(0), 123.5);

    const std::vector<double> ret2 = parser.getFloatValues("thirdArg");
    TEST_EQUAL(ret2.size(), 1);
    TEST_EQUAL(ret2.at(0), 42.25);
}

/**
 * @brief getBoolValues_test
 */
void
ArgParser_Test::getBoolValues_test()
{
    ArgParser parser;
    prepareTest(parser);

    const std::vector<bool> ret = parser.getBoolValues("bool");
    TEST_EQUAL(ret.size(), 1);
    TEST_EQUAL(ret.at(0), true);

    const std::vector<bool> ret2 = parser.getBoolValues("lastArg");
    TEST_EQUAL(ret2.size(), 1);
    TEST_EQUAL(ret2.at(0), false);
}

/**
 * @brief getStringValue_test
 */
void
ArgParser_Test::getStringValue_test()
{
    ArgParser parser;
    prepareTest(parser);

    const std::string ret2 = parser.getStringValue("first_arg");
    TEST_EQUAL(ret2, "poi");
}

/**
 * @brief getIntValue_test
 */
void
ArgParser_Test::getIntValue_test()
{
    ArgParser parser;
    prepareTest(parser);

    const long ret2 = parser.getIntValue("secondArg");
    TEST_EQUAL(ret2, 42);
}

/**
 * @brief getFloatValue_test
 */
void
ArgParser_Test::getFloatValue_test()
{
    ArgParser parser;
    prepareTest(parser);

    const double ret2 = parser.getFloatValue("thirdArg");
    TEST_EQUAL(ret2, 42.25);
}

/**
 * @brief getBoolValue_test
 */
void
ArgParser_Test::getBoolValue_test()
{
    ArgParser parser;
    prepareTest(parser);

    const bool ret2 = parser.getBoolValue("lastArg");
    TEST_EQUAL(ret2, false);
}

/**
 * @brief ArgParser_Test::prepareTest
 * @param parser
 */
void
ArgParser_Test::prepareTest(ArgParser& parser)
{
    ErrorContainer error;

    int argc = 15;
    const char* argv[15];
    argv[0] = "-";
    argv[1] = "--test";
    argv[2] = "test1";
    argv[3] = "--test";
    argv[4] = "test2";
    argv[5] = "--integer";
    argv[6] = "1337";
    argv[7] = "-f";
    argv[8] = "123.5";
    argv[9] = "-b";
    argv[10] = "true";
    argv[11] = "poi";
    argv[12] = "42";
    argv[13] = "42.25";
    argv[14] = "false";

    parser.registerString("test");
    parser.registerInteger("integer", 'i');
    parser.registerFloat("float", 'f');
    parser.registerBoolean("bool", 'b');
    parser.registerString("first_arg").setHelpText("first argument").setRequired().setWithoutFlag();
    parser.registerInteger("secondArg")
        .setHelpText("second argument")
        .setRequired()
        .setWithoutFlag();
    parser.registerFloat("thirdArg").setHelpText("third argument").setRequired().setWithoutFlag();
    parser.registerBoolean("lastArg").setHelpText("last argument").setRequired().setWithoutFlag();

    parser.parse(argc, argv, error);
}

}  // namespace Hanami
