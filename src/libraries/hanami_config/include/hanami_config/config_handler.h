/**
 * @file       config_handler.h
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

#ifndef CONFIG_HANDLER_H
#define CONFIG_HANDLER_H

#include <hanami_common/logger.h>

#include <iostream>
#include <map>
#include <nlohmann/json.hpp>
#include <vector>

using json = nlohmann::json;

#define INIT_CONFIG Hanami::ConfigHandler::getInstance()->initConfig

#define REGISTER_STRING_CONFIG Hanami::ConfigHandler::getInstance()->registerString
#define REGISTER_INT_CONFIG Hanami::ConfigHandler::getInstance()->registerInteger
#define REGISTER_FLOAT_CONFIG Hanami::ConfigHandler::getInstance()->registerFloat
#define REGISTER_BOOL_CONFIG Hanami::ConfigHandler::getInstance()->registerBoolean
#define REGISTER_STRING_ARRAY_CONFIG Hanami::ConfigHandler::getInstance()->registerStringArray

#define GET_STRING_CONFIG Hanami::ConfigHandler::getInstance()->getString
#define GET_INT_CONFIG Hanami::ConfigHandler::getInstance()->getInteger
#define GET_FLOAT_CONFIG Hanami::ConfigHandler::getInstance()->getFloat
#define GET_BOOL_CONFIG Hanami::ConfigHandler::getInstance()->getBoolean
#define GET_STRING_ARRAY_CONFIG Hanami::ConfigHandler::getInstance()->getStringArray

namespace Hanami
{
class IniItem;

class ConfigHandler_Test;

//==================================================================================================

class ConfigHandler
{
   public:
    static ConfigHandler* getInstance()
    {
        if (instance == nullptr) {
            instance = new ConfigHandler();
        }
        return instance;
    }

    struct ConfigDef {
        enum ConfigType {
            UNDEFINED_TYPE,
            STRING_TYPE,
            INT_TYPE,
            FLOAT_TYPE,
            BOOL_TYPE,
            STRING_ARRAY_TYPE
        };

        bool isRequired = false;
        ConfigType type = UNDEFINED_TYPE;
        json value;
        std::string comment = "";

        ConfigDef& setComment(const std::string& comment)
        {
            this->comment = comment;
            return *this;
        }

        ConfigDef& setDefault(const json& defaultValue)
        {
            this->value = defaultValue;
            return *this;
        }

        ConfigDef& setRequired()
        {
            this->isRequired = true;
            return *this;
        }
    };

    bool initConfig(const std::string& configFilePath, ErrorContainer& error);
    void createDocumentation(std::string& docu);

    // register config-options
    ConfigDef& registerString(const std::string& groupName, const std::string& itemName);
    ConfigDef& registerInteger(const std::string& groupName, const std::string& itemName);
    ConfigDef& registerFloat(const std::string& groupName, const std::string& itemName);
    ConfigDef& registerBoolean(const std::string& groupName, const std::string& itemName);
    ConfigDef& registerStringArray(const std::string& groupName, const std::string& itemName);

    // getter
    const std::string getString(const std::string& groupName,
                                const std::string& itemName,
                                bool& success);
    long getInteger(const std::string& groupName, const std::string& itemName, bool& success);
    double getFloat(const std::string& groupName, const std::string& itemName, bool& success);
    bool getBoolean(const std::string& groupName, const std::string& itemName, bool& success);
    const std::vector<std::string> getStringArray(const std::string& groupName,
                                                  const std::string& itemName,
                                                  bool& success);

    static Hanami::ConfigHandler* m_config;

   private:
    friend ConfigHandler_Test;

    ConfigHandler();
    ~ConfigHandler();
    static ConfigHandler* instance;

    bool checkEntry(const std::string& groupName,
                    const std::string& itemName,
                    ConfigDef& entry,
                    ErrorContainer& error);
    bool checkType(const std::string& groupName,
                   const std::string& itemName,
                   const ConfigDef::ConfigType type);
    bool isRegistered(const std::string& groupName, const std::string& itemName);

    ConfigDef::ConfigType getRegisteredType(const std::string& groupName,
                                            const std::string& itemName);

    ConfigDef& registerValue(const std::string& groupName,
                             const std::string& itemName,
                             const ConfigDef::ConfigType type);

    std::string m_configFilePath = "";
    IniItem* m_iniItem = nullptr;
    std::map<std::string, std::map<std::string, ConfigDef>> m_registeredConfigs;
};

}  // namespace Hanami

#endif  // CONFIG_HANDLER_H
