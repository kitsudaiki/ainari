/**
 * @file    ini_item.h
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

#ifndef INIITEM_H
#define INIITEM_H

#include <hanami_common/logger.h>

#include <string>
#include <vector>

namespace Hanami
{
class IniParserInterface;

class IniItem
{
   public:
    IniItem();
    ~IniItem();

    bool parse(const std::string& content, ErrorContainer& error);

    // get
    bool get(json& result, const std::string& group, const std::string& item);

    // set
    bool set(const std::string& group,
             const std::string& item,
             const char* value,
             const bool force = false);
    bool set(const std::string& group,
             const std::string& item,
             const std::string value,
             const bool force = false);
    bool set(const std::string& group,
             const std::string& item,
             const long value,
             const bool force = false);
    bool set(const std::string& group,
             const std::string& item,
             const double value,
             const bool force = false);
    bool set(const std::string& group,
             const std::string& item,
             const bool value,
             const bool force = false);
    bool set(const std::string& group,
             const std::string& item,
             const std::vector<std::string> value,
             const bool force = false);

    // remove
    bool removeGroup(const std::string& group);
    bool removeEntry(const std::string& group, const std::string& item);

    // output
    const std::string toString();

    json m_content = nullptr;

   private:
    bool setVal(const std::string& group,
                const std::string& item,
                const json& value,
                bool force = false);
};

}  // namespace Hanami

#endif  // INIITEM_H
