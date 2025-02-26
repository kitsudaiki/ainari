/**
 * @file    ini_parser_interface.h
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
#ifndef INIPARSERINTERFACE_H
#define INIPARSERINTERFACE_H

#include <hanami_common/logger.h>

#include <map>
#include <mutex>
#include <string>

using std::map;
using std::pair;
using std::string;

#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace Hanami
{
class location;

class IniParserInterface
{
   public:
    static IniParserInterface* getInstance();

    // connection the the scanner and parser
    void scan_begin(const std::string& inputString);
    void scan_end();
    json parse(const std::string& inputString, ErrorContainer& error);
    const std::string removeQuotes(const std::string& input);

    // output-handling
    void setOutput(json output);

    // Error handling.
    void error(const Hanami::location& location, const std::string& message);

    // static variables, which are used in lexer and parser
    static bool m_outsideComment;

   private:
    IniParserInterface(const bool traceParsing = false);

    static IniParserInterface* m_instance;

    json m_output;
    std::string m_errorMessage = "";
    std::string m_inputString = "";
    std::mutex m_lock;

    bool m_traceParsing = false;
};

}  // namespace Hanami

#endif  // INIPARSERINTERFACE_H
