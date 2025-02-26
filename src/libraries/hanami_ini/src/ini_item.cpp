/**
 * @file    ini_item.cpp
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

#include <hanami_ini/ini_item.h>
#include <ini_parsing/ini_parser_interface.h>

namespace Hanami
{

/**
 * constructor
 */
IniItem::IniItem() { m_content = json::object(); }

/**
 * destructor
 */
IniItem::~IniItem() {}

/**
 * @brief parse the content of an ini-file
 *
 * @param content content of an ini-file as string
 * @param errorMessage reference for error-message output
 *
 * @return true, if successful, else false
 */
bool
IniItem::parse(const std::string& content, ErrorContainer& error)
{
    IniParserInterface* parser = IniParserInterface::getInstance();

    // parse ini-template into a data-tree
    m_content = parser->parse(content, error);
    if (m_content.size() == 0) {
        LOG_ERROR(error);
        return false;
    }

    return true;
}

/**
 * @brief get a value from the ini-item
 *
 * @param group group-name
 * @param item item-key-name
 *
 * @return requested value as data-item, if found, else nullptr
 */
bool
IniItem::get(json& result, const std::string& group, const std::string& item)
{
    if (m_content.contains(group) == false) {
        return false;
    }

    json groupContent = m_content[group];
    if (groupContent.contains(item) == false) {
        return false;
    }

    result = groupContent[item];

    return true;
}

/**
 * @brief set a value inside the ini-items
 *
 * @param group group-name
 * @param item item-key-name
 * @param value char-pointer as new Value
 * @param force overwrite, if already exist
 *
 * @return false, if item already exist with value and force is false, else it returns true
 */
bool
IniItem::set(const std::string& group, const std::string& item, const char* value, const bool force)
{
    const std::string stringVal = std::string(value);
    return setVal(group, item, stringVal, force);
}

/**
 * @brief set a value inside the ini-items
 *
 * @param group group-name
 * @param item item-key-name
 * @param value string as new Value
 * @param force overwrite, if already exist
 *
 * @return false, if item already exist with value and force is false, else it returns true
 */
bool
IniItem::set(const std::string& group,
             const std::string& item,
             const std::string value,
             const bool force)
{
    return setVal(group, item, value, force);
}

/**
 * @brief set a value inside the ini-items
 *
 * @param group group-name
 * @param item item-key-name
 * @param value long as new Value
 * @param force overwrite, if already exist
 *
 * @return false, if item already exist with value and force is false, else it returns true
 */
bool
IniItem::set(const std::string& group, const std::string& item, const long value, const bool force)
{
    return setVal(group, item, value, force);
}

/**
 * @brief set a value inside the ini-items
 *
 * @param group group-name
 * @param item item-key-name
 * @param value double new Value
 * @param force overwrite, if already exist
 *
 * @return false, if item already exist with value and force is false, else it returns true
 */
bool
IniItem::set(const std::string& group,
             const std::string& item,
             const double value,
             const bool force)
{
    return setVal(group, item, value, force);
}

/**
 * @brief set a value inside the ini-items
 *
 * @param group group-name
 * @param item item-key-name
 * @param value bool new Value
 * @param force overwrite, if already exist
 *
 * @return false, if item already exist with value and force is false, else it returns true
 */
bool
IniItem::set(const std::string& group, const std::string& item, const bool value, const bool force)
{
    return setVal(group, item, value, force);
}

/**
 * @brief set a value inside the ini-items
 *
 * @param group group-name
 * @param item item-key-name
 * @param value string-list as new value
 * @param force overwrite, if already exist
 *
 * @return false, if item already exist with value and force is false, else it returns true
 */
bool
IniItem::set(const std::string& group,
             const std::string& item,
             const std::vector<std::string> value,
             const bool force)
{
    json array = json::array();
    for (uint64_t i = 0; i < value.size(); i++) {
        array.push_back(value.at(i));
    }

    return setVal(group, item, array, force);
}

/**
 * @brief remove an entire group together with all its items from the ini-item
 *
 * @param group group-name
 *
 * @return false, if group doesn't exist, else true
 */
bool
IniItem::removeGroup(const std::string& group)
{
    return m_content.erase(group);
}

/**
 * @brief remove an item for the ini-item
 *
 * @param group group-name
 * @param item item-key-name
 *
 * @return false, if group or item doesn't exist, else true
 */
bool
IniItem::removeEntry(const std::string& group, const std::string& item)
{
    if (m_content.contains(group) == false) {
        return false;
    }

    m_content[group].erase(item);
    return true;
}

/**
 * @brief set a value inside the ini-items
 *
 * @param group group-name
 * @param item item-key-name
 * @param value new Value
 * @param force overwrite, if already exist
 *
 * @return false, if item already exist with value and force is false, else it returns true
 */
bool
IniItem::setVal(const std::string& group,
                const std::string& item,
                const json& value,
                const bool force)
{
    // if group doesn't exist, create the group with the new content
    if (m_content.contains(group) == false) {
        json groupContent = json::object();
        groupContent[item] = value;
        m_content[group] = groupContent;
        return true;
    }

    // item doesn't exist or should be overrided by force
    if (m_content[group].contains(item) == false || force) {
        m_content[group][item] = value;
        return true;
    }

    return false;
}

/**
 * @brief convert the content of the ini-item into a string-output to write it into a new ini-file
 *
 * @return converted string
 */
const std::string
IniItem::toString()
{
    std::string output = "";

    // iterate over all groups
    for (const auto& [name, globalItem] : m_content.items()) {
        // print group-header
        output.append("[");
        output.append(name);
        output.append("]\n");

        // iterate over group-content
        for (const auto& [name, groupItem] : globalItem.items()) {
            // print line of group-content
            output.append(name);
            output.append(" = ");

            if (groupItem.is_array()) {
                // print arrays
                for (uint64_t i = 0; i < groupItem.size(); i++) {
                    if (i != 0) {
                        output.append(", ");
                    }
                    if (groupItem[i].is_string()) {
                        output.append(groupItem[i]);
                    }
                    else {
                        output.append(groupItem[i].dump());
                    }
                }
            }
            else {
                // print simple items
                if (groupItem.is_string()) {
                    output.append(groupItem);
                }
                else {
                    output.append(groupItem.dump());
                }
            }

            output.append("\n");
        }

        output.append("\n");
    }

    return output;
}

}  // namespace Hanami
