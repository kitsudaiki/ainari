/**
 * @file       main.cpp
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

#include <hanami_args/arg_parser.h>
#include <hanami_common/logger.h>

int
main(int argc, char* argv[])
{
    // error messages of the parser are printed via logger
    Hanami::initConsoleLogger(true);

    Hanami::ArgParser parser("0.1.0");

    Hanami::ErrorContainer error;

    // register flags without value
    parser.registerPlain("debug", 'd').setHelpText("debug-flag to enable addtional debug output");
    // "registerPlain" allows it to register flags without any value, which says only true or flase
    //                 if they were set or not set

    // register flags
    parser.registerString("source").setHelpText("source-path").setRequired();
    parser.registerInteger("input", 'i').setHelpText("additional parameter");

    // register other values
    parser.registerString("mode")
        .setHelpText("modus for converting")
        .setRequired()
        .setWithoutFlag();
    parser.registerString("destination")
        .setHelpText("destination path for output")
        .setRequired()
        .setWithoutFlag();

    // register types:
    //     registerString
    //     registerInteger
    //     registerFloat
    //     registerBoolean

    bool ret = parser.parse(argc, argv, error);
    if (ret == false) {
        return 1;
    }
    // ret say, if the converting was successful or not. Error-message are written in the logger

    // check if flags without values were set. In this case check if the debug-flag was set

    // get values with or without flag as list of value for the case, that a flag was
    // used multiple times within one cli-call:
    const std::vector<std::string> testValues = parser.getStringValues("source");
    const std::vector<long> numbers = parser.getIntValues("input");
    // get types:
    //     getStringValues
    //     getIntValues
    //     getFloatValues
    //     getBoolValues

    // get values without flag:
    const std::string mode = parser.getStringValue("mode");
    const std::string destination = parser.getStringValue("destination");
    // get types:
    //     getStringValue
    //     getIntValue
    //     getFloatValue
    //     getBoolValue

    //...

    return 0;
}
