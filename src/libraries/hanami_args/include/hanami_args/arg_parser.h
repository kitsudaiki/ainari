/**
 * @file       arg_parser.h
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

#ifndef ARG_PARSER_H
#define ARG_PARSER_H

#include <hanami_common/logger.h>

#include <climits>
#include <cstdlib>
#include <iostream>
#include <vector>

namespace Hanami
{
class ArgParser_Test;
class SubCommand;

class ArgParser
{
   public:
    struct ArgDef {
        enum ArgType { NO_TYPE, STRING_TYPE, INT_TYPE, FLOAT_TYPE, BOOL_TYPE };

        const std::string longIdentifier;
        const std::string shortIdentifier;
        ArgType type = NO_TYPE;
        std::string helpText = "";
        bool withoutFlag = false;
        bool isRequired = false;
        bool hasValue = false;

        json results = json::array();
        bool wasSet = false;

        ArgDef(const std::string& longIdent, const char shortIdent = ' ')
            : longIdentifier("--" + longIdent), shortIdentifier("-" + std::string{shortIdent})
        {
        }

        ArgDef& setHelpText(const std::string& helpText)
        {
            this->helpText = helpText;
            return *this;
        }

        ArgDef& setRequired()
        {
            this->isRequired = true;
            return *this;
        }

        ArgDef& setWithoutFlag()
        {
            this->withoutFlag = true;
            return *this;
        }
    };

    ArgParser(const std::string& version = "");
    ~ArgParser();

    // register
    ArgDef& registerPlain(const std::string& longIdent, const char shortIdent = ' ');
    ArgDef& registerString(const std::string& longIdent, const char shortIdent = ' ');
    ArgDef& registerInteger(const std::string& longIdent, const char shortIdent = ' ');
    ArgDef& registerFloat(const std::string& longIdent, const char shortIdent = ' ');
    ArgDef& registerBoolean(const std::string& longIdent, const char shortIdent = ' ');

    // parse
    bool parse(const int argc, char* argv[], ErrorContainer& error);
    bool parse(const int argc, const char* argv[], ErrorContainer& error);

    // getter
    uint64_t getNumberOfValues(const std::string& identifier);
    bool wasSet(const std::string& identifier);
    const std::vector<std::string> getStringValues(const std::string& identifier);
    const std::vector<long> getIntValues(const std::string& identifier);
    const std::vector<double> getFloatValues(const std::string& identifier);
    const std::vector<bool> getBoolValues(const std::string& identifier);

    const std::string getStringValue(const std::string& identifier);
    long getIntValue(const std::string& identifier);
    double getFloatValue(const std::string& identifier);
    bool getBoolValue(const std::string& identifier);

   private:
    friend ArgParser_Test;
    friend SubCommand;

    uint32_t m_positionCounter = 0;
    std::string m_version = "";
    std::vector<ArgDef> m_argumentList;

    const std::string convertType(ArgDef::ArgType type);
    void print(const std::string& commandName);
    bool precheckFlags(const int argc, const char* argv[]);

    ArgDef* getArgument(const std::string& identifier);
    int32_t registerArgument(ArgDef& newArgument);

    json convertValue(const std::string& value, const ArgDef::ArgType requiredType);
};

}  // namespace Hanami

#endif  // ARG_PARSER_H
