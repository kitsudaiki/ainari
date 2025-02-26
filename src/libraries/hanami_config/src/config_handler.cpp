/**
 * @file       config_handler.cpp
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

#include <hanami_common/files/text_file.h>
#include <hanami_config/config_handler.h>
#include <hanami_ini/ini_item.h>

namespace Hanami
{

ConfigHandler* ConfigHandler::instance = nullptr;

/**
 * @brief ConfigHandler::ConfigHandler
 */
ConfigHandler::ConfigHandler() {}

/**
 * @brief ConfigHandler::~ConfigHandler
 */
ConfigHandler::~ConfigHandler() { delete m_iniItem; }

/**
 * @brief read a ini config-file
 *
 * @param configFilePath absolute path to the config-file to read
 * @param error reference for error-output
 *
 * @return false, if reading or parsing the file failed, else true
 */
bool
ConfigHandler::initConfig(const std::string& configFilePath, ErrorContainer& error)
{
    // read file
    m_configFilePath = configFilePath;
    std::string fileContent = "";
    const bool ret = readFile(fileContent, m_configFilePath, error);
    if (ret == false) {
        error.addMessage("Error while reading config-file \"" + configFilePath + "\"");
        LOG_ERROR(error);
        return false;
    }

    // parse file content
    m_iniItem = new IniItem();
    std::string parseErrorMessage = "";
    const bool result = m_iniItem->parse(fileContent, error);
    if (result == false) {
        error.addMessage("Error while parsing config-file \"" + configFilePath + "\"");
        LOG_ERROR(error);
        return false;
    }

    // check config against the registered entries
    for (auto& [groupName, groupConfig] : m_registeredConfigs) {
        for (auto& [itemName, entry] : groupConfig) {
            if (checkEntry(groupName, itemName, entry, error) == false) {
                error.addMessage("Error while checking config-file \"" + configFilePath + "\"");
                LOG_ERROR(error);
                return false;
            }
        }
    }

    return true;
}

/**
 * @brief register string config value
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 *
 * @return false, if item-name and group-name are already registered, else true
 */
ConfigHandler::ConfigDef&
ConfigHandler::registerString(const std::string& groupName, const std::string& itemName)
{
    return registerValue(groupName, itemName, ConfigDef::STRING_TYPE).setDefault("");
}

/**
 * @brief register int/long config value
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 *
 * @return false, if item-name and group-name are already registered, else true
 */
ConfigHandler::ConfigDef&
ConfigHandler::registerInteger(const std::string& groupName, const std::string& itemName)
{
    return registerValue(groupName, itemName, ConfigDef::INT_TYPE).setDefault(0);
}

/**
 * @brief register float/double config value
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 *
 * @return false, if item-name and group-name are already registered, else true
 */
ConfigHandler::ConfigDef&
ConfigHandler::registerFloat(const std::string& groupName, const std::string& itemName)
{
    return registerValue(groupName, itemName, ConfigDef::FLOAT_TYPE).setDefault(0.0f);
}

/**
 * @brief register bool config value
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 *
 * @return false, if item-name and group-name are already registered, else true
 */
ConfigHandler::ConfigDef&
ConfigHandler::registerBoolean(const std::string& groupName, const std::string& itemName)
{
    return registerValue(groupName, itemName, ConfigDef::BOOL_TYPE).setDefault(false);
}

/**
 * @brief register string-array config value
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 *
 * @return false, if item-name and group-name are already registered, else true
 */
ConfigHandler::ConfigDef&
ConfigHandler::registerStringArray(const std::string& groupName, const std::string& itemName)
{
    return registerValue(groupName, itemName, ConfigDef::STRING_ARRAY_TYPE);
}

/**
 * @brief get string-value from config
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 * @param success reference to bool-value with the result. returns false if item-name and group-name
 *                are not registered, else true.
 *
 * @return empty string, if item-name and group-name are not registered, else value from the
 *         config-file or the defined default-value.
 */
const std::string
ConfigHandler::getString(const std::string& groupName, const std::string& itemName, bool& success)
{
    success = true;

    // compare with registered type
    if (getRegisteredType(groupName, itemName) != ConfigDef::STRING_TYPE) {
        success = false;
        return "";
    }

    // get value from config
    return m_registeredConfigs[groupName][itemName].value;
}

/**
 * @brief get long-value from config
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 * @param success reference to bool-value with the result. returns false if item-name and group-name
 *                are not registered, else true.
 *
 * @return 0, if item-name and group-name are not registered, else value from the
 *         config-file or the defined default-value.
 */
long
ConfigHandler::getInteger(const std::string& groupName, const std::string& itemName, bool& success)
{
    success = true;

    // compare with registered type
    if (getRegisteredType(groupName, itemName) != ConfigDef::INT_TYPE) {
        success = false;
        return 0l;
    }

    // get value from config
    return m_registeredConfigs[groupName][itemName].value;
}

/**
 * @brief get double-value from config
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 * @param success reference to bool-value with the result. returns false if item-name and group-name
 *                are not registered, else true.
 *
 * @return 0.0, if item-name and group-name are not registered, else value from the
 *         config-file or the defined default-value.
 */
double
ConfigHandler::getFloat(const std::string& groupName, const std::string& itemName, bool& success)
{
    success = true;

    // compare with registered type
    if (getRegisteredType(groupName, itemName) != ConfigDef::FLOAT_TYPE) {
        success = false;
        return 0.0;
    }

    // get value from config
    return m_registeredConfigs[groupName][itemName].value;
}

/**
 * @brief get bool-value from config
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 * @param success reference to bool-value with the result. returns false if item-name and group-name
 *                are not registered, else true.
 *
 * @return false, if item-name and group-name are not registered, else value from the
 *         config-file or the defined default-value.
 */
bool
ConfigHandler::getBoolean(const std::string& groupName, const std::string& itemName, bool& success)
{
    success = true;

    // compare with registered type
    if (getRegisteredType(groupName, itemName) != ConfigDef::BOOL_TYPE) {
        success = false;
        return false;
    }

    // get value from config
    return m_registeredConfigs[groupName][itemName].value;
}

/**
 * @brief get string-array-value from config
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 * @param success reference to bool-value with the result. returns false if item-name and group-name
 *                are not registered, else true.
 *
 * @return empty string-array, if item-name and group-name are not registered, else value from the
 *         config-file or the defined default-value.
 */
const std::vector<std::string>
ConfigHandler::getStringArray(const std::string& groupName,
                              const std::string& itemName,
                              bool& success)
{
    std::vector<std::string> result;
    success = true;

    // compare with registered type
    if (getRegisteredType(groupName, itemName) != ConfigDef::STRING_ARRAY_TYPE) {
        success = false;
        return result;
    }

    // get and transform result from the config-file
    json array = m_registeredConfigs[groupName][itemName].value;
    for (uint32_t i = 0; i < array.size(); i++) {
        result.push_back(array[i]);
    }

    return result;
}

/**
 * @brief check entry against the read config-file and override the default
 *        value if set by the config
 *
 * @param groupName group-name of the entry to check
 * @param itemName item-name of the entry to check
 * @param entry entry to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
ConfigHandler::checkEntry(const std::string& groupName,
                          const std::string& itemName,
                          ConfigDef& entry,
                          ErrorContainer& error)
{
    // check type against config-file
    if (checkType(groupName, itemName, entry.type) == false) {
        error.addMessage("Config registration failed because item has the false value type: \n"
                         "    group: \'" + groupName + "\'\n"
                         "    item: \'" + itemName + "\'");
        return false;
    }

    // check if value is required
    json currentVal;
    const bool found = m_iniItem->get(currentVal, groupName, itemName);
    if (entry.isRequired && found == false) {
        error.addMessage("Config registration failed because required "
                         "value was not set in the config: \n"
                         "    group: \'" + groupName + "\'\n"
                         "    item: \'" + itemName + "\'");
        return false;
    }

    // overwrite the registered default-value with the value of the config
    if (currentVal.size() != 0) {
        entry.value = currentVal;
    }

    return true;
}

/**
 * @brief check if defined type match with the type of the value within the config-file
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 * @param type type-identifier
 *
 * @return true, if type match with the config-file, else false
 */
bool
ConfigHandler::checkType(const std::string& groupName,
                         const std::string& itemName,
                         const ConfigDef::ConfigType type)
{
    // get value from config-file
    json currentItem;
    const bool found = m_iniItem->get(currentItem, groupName, itemName);
    if (found == false) {
        return true;
    }

    // precheck
    if (currentItem == nullptr) {
        return true;
    }

    // check for array
    if (currentItem.is_array() && type == ConfigDef::STRING_ARRAY_TYPE) {
        return true;
    }

    // check value
    if (currentItem.is_array() == false && currentItem.is_object() == false) {
        // check string
        if (currentItem.is_string() && type == ConfigDef::STRING_TYPE) {
            return true;
        }

        // check integer
        if (currentItem.is_number_integer() && type == ConfigDef::INT_TYPE) {
            return true;
        }

        // check float
        if (currentItem.is_number_float() && type == ConfigDef::FLOAT_TYPE) {
            return true;
        }

        // check boolean
        if (currentItem.is_boolean() && type == ConfigDef::BOOL_TYPE) {
            return true;
        }
    }

    return false;
}

/**
 * @brief check, if an item-name and group-name are already registered
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 *
 * @return true, if item-name and group-name is already registered, else false
 */
bool
ConfigHandler::isRegistered(const std::string& groupName, const std::string& itemName)
{
    const auto outerIt = m_registeredConfigs.find(groupName);
    if (outerIt == m_registeredConfigs.end()) {
        return false;
    }

    const auto innerIt = outerIt->second.find(itemName);
    if (innerIt == outerIt->second.end()) {
        return false;
    }

    return true;
}

/**
 * @brief get registered config-type
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 *
 * @return undefined-type, if item-name and group-name are not registered, else the registered type
 */
ConfigHandler::ConfigDef::ConfigType
ConfigHandler::getRegisteredType(const std::string& groupName, const std::string& itemName)
{
    const auto outerIt = m_registeredConfigs.find(groupName);
    if (outerIt != m_registeredConfigs.end()) {
        const auto innerIt = outerIt->second.find(itemName);
        if (innerIt != outerIt->second.end()) {
            return innerIt->second.type;
        }
    }

    return ConfigDef::UNDEFINED_TYPE;
}

/**
 * @brief register single value in the config
 *
 * @param groupName name of the group
 * @param itemName name of the item within the group
 * @param type type of the value to register
 * @param error reference for error-output
 *
 * @return true, if successfull, else false
 */
ConfigHandler::ConfigDef&
ConfigHandler::registerValue(const std::string& groupName,
                             const std::string& itemName,
                             const ConfigDef::ConfigType type)
{
    std::string finalGroupName = groupName;

    // if group-name is empty, then use the default-group
    if (finalGroupName.size() == 0) {
        finalGroupName = "DEFAULT";
    }

    // precheck if already exist
    if (isRegistered(finalGroupName, itemName) == true) {
        ErrorContainer error;
        error.addMessage("Config registration failed because item is already registered: \n"
                         "    group: \'" + finalGroupName + "\'\n"
                         "    item: \'" + itemName + "\'");
        LOG_ERROR(error);
        assert(false);
    }

    // add groupName, if not exist
    if (m_registeredConfigs.find(finalGroupName) == m_registeredConfigs.end()) {
        std::map<std::string, ConfigDef> newEntry;
        m_registeredConfigs.try_emplace(finalGroupName, newEntry);
    }

    // add new value
    auto outerIt = m_registeredConfigs.find(finalGroupName);
    ConfigDef entry;
    entry.type = type;
    auto it = outerIt->second.try_emplace(itemName, entry);

    return it.first->second;
}

/**
 * @brief generate markdown-text with all registered config-entries
 *
 * @param docu reference for the output of the final document
 */
void
ConfigHandler::createDocumentation(std::string& docu)
{
    for (auto& [groupName, groupConfig] : m_registeredConfigs) {
        docu.append("## " + groupName + "\n\n");
        docu.append("| Item | Description |\n");
        docu.append("| --- | --- |\n");

        for (auto& [itemName, entry] : groupConfig) {
            docu.append("| " + itemName + "| ");
            docu.append("**Description**: " + entry.comment + "<br>");
            if (entry.isRequired) {
                docu.append("**Required**: TRUE<br>");
            }
            else {
                docu.append("**Required**: FALSE<br>");
            }
            if (entry.value != nullptr && entry.isRequired == false) {
                const std::string value = entry.value.dump();
                docu.append("**Default**: " + value + "<br>");
            }
            docu.append(" |\n");
        }
        docu.append("\n");
    }
}

}  // namespace Hanami
