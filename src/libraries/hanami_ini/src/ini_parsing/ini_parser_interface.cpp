/**
 * @file    ini_parser_interface.cpp
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

#include <hanami_common/functions/string_functions.h>
#include <ini_parser.h>
#include <ini_parsing/ini_parser_interface.h>

#define YY_DECL Hanami::IniParser::symbol_type inilex(Hanami::IniParserInterface& driver)
YY_DECL;

namespace Hanami
{

Hanami::IniParserInterface* IniParserInterface::m_instance = nullptr;

using Hanami::splitStringByDelimiter;

bool IniParserInterface::m_outsideComment = true;

/**
 * The class is the interface for the bison-generated parser.
 * It starts the parsing-process and store the returned values.
 *
 * @param traceParsing If set to true, the scanner prints all triggered rules.
 *                     It is only for better debugging.
 */
IniParserInterface::IniParserInterface(const bool traceParsing) { m_traceParsing = traceParsing; }

/**
 * @brief static methode to get instance of the interface
 *
 * @return pointer to the static instance
 */
IniParserInterface*
IniParserInterface::getInstance()
{
    if (m_instance == nullptr) {
        m_instance = new IniParserInterface();
    }

    return m_instance;
}

/**
 * @brief parse string
 *
 * @param inputString string which should be parsed
 * @param error reference for error-message
 *
 * @return resulting object
 */
json
IniParserInterface::parse(const std::string& inputString, ErrorContainer& error)
{
    std::lock_guard<std::mutex> guard(m_lock);

    // init global values
    m_inputString = inputString;
    m_errorMessage = "";

    // run parser-code
    this->scan_begin(inputString);
    Hanami::IniParser parser(*this);
    int res = parser.parse();
    this->scan_end();

    // handle negative result
    if (res != 0 || m_errorMessage.size() > 0) {
        error.addMessage(m_errorMessage);
        LOG_ERROR(error);
        return nullptr;
    }

    return m_output;
}

/**
 * @brief remove double quotes from the beginning and the end of a string
 *
 * @param input string to clear
 *
 * @return cleared string
 */
const std::string
IniParserInterface::removeQuotes(const std::string& input)
{
    // precheck
    if (input.length() == 0) {
        return input;
    }

    // clear
    if (input[0] == '\"' && input[input.length() - 1] == '\"') {
        std::string result = "";
        for (uint32_t i = 1; i < input.length() - 1; i++) {
            result += input[i];
        }

        return result;
    }

    return input;
}

/**
 * Is called for the parser after successfully parsing the input-string
 *
 * @param output parser-output as DataArray
 */
void
IniParserInterface::setOutput(json output)
{
    m_output = output;
}

/**
 * Is called from the parser in case of an error
 *
 * @param location location-object of the bison-parser,
 *                 which contains the informations of the location
 *                 of the syntax-error in the parsed string
 * @param message error-specific message from the parser
 */
void
IniParserInterface::error(const location& location, const std::string& message)
{
    if (m_errorMessage.size() > 0) {
        return;
    }

    // get the broken part of the parsed string
    const uint32_t errorStart = location.begin.column;
    const uint32_t errorLength = location.end.column - location.begin.column;
    const uint32_t linenumber = location.begin.line;

    std::vector<std::string> splittedContent;
    splitStringByDelimiter(splittedContent, m_inputString, '\n');

    // build error-message
    m_errorMessage = "ERROR while parsing ini-formated string \n";
    m_errorMessage += "parser-message: " + message + " \n";
    m_errorMessage += "line-number: " + std::to_string(linenumber) + " \n";

    if (splittedContent[linenumber - 1].size() > errorStart - 1 + errorLength) {
        m_errorMessage.append("position in line: " + std::to_string(location.begin.column) + "\n");
        m_errorMessage.append("broken part in string: \""
                              + splittedContent[linenumber - 1].substr(errorStart - 1, errorLength)
                              + "\"");
    }
    else {
        m_errorMessage.append("position in line: UNKNOWN POSITION (maybe a string was not closed)");
    }
}

}  // namespace Hanami
