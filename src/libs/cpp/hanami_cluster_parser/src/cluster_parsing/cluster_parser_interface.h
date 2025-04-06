/**
 * @file       cluster_parser_interface.h
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

#ifndef HANAMI_SEGMENT_PARSER_PARSER_INTERFACE_H
#define HANAMI_SEGMENT_PARSER_PARSER_INTERFACE_H

#include <hanami_common/logger.h>

#include <mutex>

struct Position;

namespace Hanami
{
struct ClusterMeta;
class location;

class ClusterParserInterface
{
   public:
    static ClusterParserInterface* getInstance();
    ~ClusterParserInterface();

    // connection the the scanner and parser
    void scan_begin(const std::string& inputString);
    void scan_end();
    bool parse(ClusterMeta* result, const std::string& inputString, ErrorContainer& error);
    const std::string removeQuotes(const std::string& input);
    uint32_t getHexagonId(const Position& position);

    // Error handling.
    void error(const Hanami::location& location, const std::string& message);

    ClusterMeta* output = nullptr;

   private:
    ClusterParserInterface(const bool traceParsing = false);

    static ClusterParserInterface* m_instance;

    std::string m_errorMessage = "";
    std::string m_inputString = "";
    std::mutex m_lock;

    bool m_traceParsing = false;
};

}  // namespace Hanami

#endif  // HANAMI_SEGMENT_PARSER_PARSER_INTERFACE_H
