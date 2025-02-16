/**
 * @file       arg_parser.cpp
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
#include <hanami_common/functions/string_functions.h>
#include <hanami_common/items/table_item.h>
#include <hanami_common/logger.h>

namespace Hanami
{

/**
 * @brief constructor
 */
ArgParser::ArgParser(const std::string& version)
{
    ArgDef helpArg("help", 'h');
    helpArg.setHelpText("print help ouput");

    if (version != "") {
        m_version = version;
        ArgDef helpArg("version", 'v');
        helpArg.setHelpText("print program version");
    }
}

/**
 * @brief destructor
 */
ArgParser::~ArgParser() {}

/**
 * @brief register argument without value
 *
 * @param identifier Identifier for the new argument. Its a single word like "flag" for defining
 *                   only a long identifier like "--flag" or a comma-separated pair like "flag,f"
 *                   to define a long identifier like "--flag" together with a short identifier
 *                   like "-f"
 *
 * @return false, if identifier is already registered or broken, else true
 */
ArgParser::ArgDef&
ArgParser::registerPlain(const std::string& longIdent, const char shortIdent)
{
    ArgDef newDef(longIdent, shortIdent);
    newDef.type = ArgDef::NO_TYPE;
    const int32_t pos = registerArgument(newDef);
    assert(pos != -1);
    return m_argumentList[pos];
}

/**
 * @brief register string-value
 *
 * @param longIdent long identifier
 * @param shortIdent short identifier
 *
 * @return reference to definition
 */
ArgParser::ArgDef&
ArgParser::registerString(const std::string& longIdent, const char shortIdent)
{
    ArgDef newDef(longIdent, shortIdent);
    newDef.type = ArgDef::STRING_TYPE;
    newDef.hasValue = true;
    const int32_t pos = registerArgument(newDef);
    assert(pos != -1);
    return m_argumentList[pos];
}

/**
 * @brief register int/long value
 *
 * @param longIdent long identifier
 * @param shortIdent short identifier
 *
 * @return reference to definition
 */
ArgParser::ArgDef&
ArgParser::registerInteger(const std::string& longIdent, const char shortIdent)
{
    ArgDef newDef(longIdent, shortIdent);
    newDef.type = ArgDef::INT_TYPE;
    newDef.hasValue = true;
    const int32_t pos = registerArgument(newDef);
    assert(pos != -1);
    return m_argumentList[pos];
}

/**
 * @brief register float/double value
 *
 * @param longIdent long identifier
 * @param shortIdent short identifier
 *
 * @return reference to definition
 */
ArgParser::ArgDef&
ArgParser::registerFloat(const std::string& longIdent, const char shortIdent)
{
    ArgDef newDef(longIdent, shortIdent);
    newDef.type = ArgDef::FLOAT_TYPE;
    newDef.hasValue = true;
    const int32_t pos = registerArgument(newDef);
    assert(pos != -1);
    return m_argumentList[pos];
}

/**
 * @brief register bool value
 *
 * @param longIdent long identifier
 * @param shortIdent short identifier
 *
 * @return reference to definition
 */
ArgParser::ArgDef&
ArgParser::registerBoolean(const std::string& longIdent, const char shortIdent)
{
    ArgDef newDef(longIdent, shortIdent);
    newDef.type = ArgDef::BOOL_TYPE;
    newDef.hasValue = true;
    const int32_t pos = registerArgument(newDef);
    assert(pos != -1);
    return m_argumentList[pos];
}

/**
 * @brief register ne argument
 *
 * @param longIdent long identifier
 * @param shortIdent short identifier
 *
 * @return reference to definition
 */
int32_t
ArgParser::registerArgument(ArgDef& newArgument)
{
    // check if already used
    ArgParser::ArgDef* findArg = nullptr;
    findArg = getArgument(newArgument.longIdentifier);
    if (findArg != nullptr) {
        return -1;
    }

    if (newArgument.shortIdentifier != "- ") {
        findArg = getArgument(newArgument.shortIdentifier);
        if (findArg != nullptr) {
            return -1;
        }
    }

    m_argumentList.push_back(newArgument);

    return m_argumentList.size() - 1;
}

/**
 * @brief convert argument-values
 *
 * @param value string-value
 * @param requiredType required type for the argument
 *
 * @return nullptr, if converting failed, else data-item with the converted value
 */
json
ArgParser::convertValue(const std::string& value, const ArgDef::ArgType requiredType)
{
    // string value
    if (requiredType == ArgDef::STRING_TYPE) {
        return (value);
    }

    // long/int value
    if (requiredType == ArgDef::INT_TYPE) {
        char* err = nullptr;
        const char* charValue = value.c_str();

        // convert to long-value
        const long longValue = std::strtol(charValue, &err, 10);
        if (std::string(err).size() != 0) {
            return nullptr;
        }

        return longValue;
    }

    // double/floag value
    if (requiredType == ArgDef::FLOAT_TYPE) {
        char* err = nullptr;
        const char* charValue = value.c_str();

        // convert to double-value
        const double doubleValue = std::strtod(charValue, &err);
        if (std::string(err).size() != 0) {
            return nullptr;
        }

        return doubleValue;
    }

    // bool value
    if (requiredType == ArgDef::BOOL_TYPE) {
        // convert true
        if (value == "true" || value == "True" || value == "1") {
            return true;
        }

        // convert false
        if (value == "false" || value == "False" || value == "0") {
            return false;
        }

        // if nothing match, it is no boolean
        return nullptr;
    }

    return nullptr;
}

/**
 * @brief precheck for the flags to handle --help and --version
 *
 * @param argc number of arguments
 * @param argv arguments
 *
 * @return true, if help or version flag was set, else true
 */
bool
ArgParser::precheckFlags(const int argc, const char* argv[])
{
    const std::string programmPath(argv[0]);
    std::vector<std::string> pathParts;
    splitStringByDelimiter(pathParts, programmPath, '/');

    for (int i = 1; i < argc; i++) {
        const std::string currentArgument(argv[i]);

        // check for help-flag
        if (currentArgument == "-h" || currentArgument == "--help") {
            print(pathParts.at(pathParts.size() - 1));
            return true;
        }

        if (m_version != "") {
            // check for version-flag
            if (currentArgument == "-v" || currentArgument == "--version") {
                std::cout << "version: " << m_version << std::endl;
                exit(0);
            }
        }
    }

    return false;
}

/**
 * @brief parse cli-arguments
 *
 * @param argc number of arguments
 * @param argv arguments
 *
 * @return false, if parsing failed
 */
bool
ArgParser::parse(const int argc, char* argv[], ErrorContainer& error)
{
    // TODO: find better solution without warning
    return parse(argc, (const char**)argv, error);
}

/**
 * @brief parse cli-arguments
 *
 * @param argc number of arguments
 * @param argv arguments as const char
 *
 * @return false, if parsing failed
 */
bool
ArgParser::parse(const int argc, const char* argv[], ErrorContainer& error)
{
    if (precheckFlags(argc, argv)) {
        return true;
    }

    m_positionCounter = 0;

    int i = 1;
    while (i < argc) {
        const std::string currentArgument(argv[i]);

        if (currentArgument.at(0) == '-') {
            // get and check identifier
            ArgParser::ArgDef* argIdent = getArgument(currentArgument);
            if (argIdent == nullptr) {
                error.addMessage("unknown argument: " + currentArgument);
                LOG_ERROR(error);
                return false;
            }

            if (argIdent->hasValue) {
                // check if there is a value for the identifier
                if (i + 1 == argc) {
                    error.addMessage("flag has no value: " + currentArgument);
                    LOG_ERROR(error);
                    return false;
                }

                // get value
                const std::string currentValue(argv[i + 1]);

                // convert value
                json convertedValue = convertValue(currentValue, argIdent->type);
                if (convertedValue == nullptr) {
                    const std::string errMsg
                        = "argument has the false type: "
                          "\n    required type: "
                          + convertType(argIdent->type) + "\n    identifier: " + currentArgument
                          + "\n    given value: " + currentValue;

                    error.addMessage(errMsg);
                    LOG_ERROR(error);
                    return false;
                }

                // add converted value to results
                argIdent->results.push_back(convertedValue);

                i += 2;
            }
            else {
                i += 1;
            }

            argIdent->wasSet = true;
        }
        else {
            uint32_t counter = 0;
            for (uint64_t j = 0; j < m_argumentList.size(); j++) {
                if (m_argumentList[j].withoutFlag == true) {
                    ArgParser::ArgDef* argIdent = &m_argumentList[j];
                    if (m_positionCounter == counter) {
                        // convert value
                        json convertedValue = convertValue(currentArgument, argIdent->type);
                        if (convertedValue == nullptr) {
                            const std::string errMsg
                                = "argument has the false type: "
                                  "\n    required type: "
                                  + convertType(argIdent->type)
                                  + "\n    identifier: " + m_argumentList[j].longIdentifier
                                  + "\n    given value: " + currentArgument;

                            error.addMessage(errMsg);
                            LOG_ERROR(error);
                            return false;
                        }

                        // add converted value to results
                        argIdent->results.push_back(convertedValue);

                        // update counter
                        m_positionCounter++;
                        j = m_argumentList.size();
                    }

                    counter++;
                }
            }

            i += 1;
        }
    }

    // check if all requirements are satisfied
    for (uint32_t i = 0; i < m_argumentList.size(); i++) {
        if (m_argumentList[i].results.size() == 0 && m_argumentList[i].isRequired) {
            error.addMessage("argument is required but was not set: "
                             + m_argumentList[i].longIdentifier);
            LOG_ERROR(error);
            return false;
        }
    }

    return true;
}

/**
 * @brief check how often a flag was set
 *
 * @param identifier identifier-name of the flag
 *
 * @return 0, if the flag was not set, else the counter of how often the flag was set
 *
 */
uint64_t
ArgParser::getNumberOfValues(const std::string& identifier)
{
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return 0;
    }

    return arg->results.size();
}

/**
 * @brief check if a flag was set
 *
 * @param identifier identifier-name of the flag
 *
 * @return true, if flag was set, else false
 */
bool
ArgParser::wasSet(const std::string& identifier)
{
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return false;
    }

    return arg->wasSet;
}

/**
 * @brief get parsed string values of an identifier
 *
 * @param identifier regested identifier without "--" or "-"
 *
 * @return list of parsed string values
 */
const std::vector<std::string>
ArgParser::getStringValues(const std::string& identifier)
{
    std::vector<std::string> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return result;
    }

    // check argument-type
    if (arg->type != ArgDef::STRING_TYPE) {
        return result;
    }

    // build list with all results
    for (uint32_t i = 0; i < arg->results.size(); i++) {
        result.push_back(arg->results[i]);
    }

    return result;
}

/**
 * @brief get parsed long values of an identifier
 *
 * @param identifier regested identifier without "--" or "-"
 *
 * @return list of parsed long values
 */
const std::vector<long>
ArgParser::getIntValues(const std::string& identifier)
{
    std::vector<long> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return result;
    }

    // check argument-type
    if (arg->type != ArgDef::INT_TYPE) {
        return result;
    }

    // build list with all results
    for (uint32_t i = 0; i < arg->results.size(); i++) {
        result.push_back(arg->results[i]);
    }

    return result;
}

/**
 * @brief get parsed double values of an identifier
 *
 * @param identifier regested identifier without "--" or "-"
 *
 * @return list of parsed double values
 */
const std::vector<double>
ArgParser::getFloatValues(const std::string& identifier)
{
    std::vector<double> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return result;
    }

    // check argument-type
    if (arg->type != ArgDef::FLOAT_TYPE) {
        return result;
    }

    // build list with all results
    for (uint32_t i = 0; i < arg->results.size(); i++) {
        result.push_back(arg->results[i]);
    }

    return result;
}

/**
 * @brief get parsed bool values of an identifier
 *
 * @param identifier regested identifier without "--" or "-"
 *
 * @return list of parsed bool values
 */
const std::vector<bool>
ArgParser::getBoolValues(const std::string& identifier)
{
    std::vector<bool> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return result;
    }

    // check argument-type
    if (arg->type != ArgDef::BOOL_TYPE) {
        return result;
    }

    // build list with all results
    for (uint32_t i = 0; i < arg->results.size(); i++) {
        result.push_back(arg->results[i]);
    }

    return result;
}

/**
 * @brief get parsed string value of an required identifier
 *
 * @param identifier regested identifier
 *
 * @return parsed string value
 */
const std::string
ArgParser::getStringValue(const std::string& identifier)
{
    std::vector<std::string> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return "";
    }

    // check argument-type
    if (arg->type != ArgDef::STRING_TYPE) {
        return "";
    }

    // check result not empty
    if (arg->results.size() == 0) {
        return "";
    }

    return arg->results[0];
}

/**
 * @brief get parsed long value of an required identifier
 *
 * @param identifier regested identifier
 *
 * @return parsed long value
 */
long
ArgParser::getIntValue(const std::string& identifier)
{
    std::vector<long> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return 0l;
    }

    // check argument-type
    if (arg->type != ArgDef::INT_TYPE) {
        return 0l;
    }

    // check result not empty
    if (arg->results.size() == 0) {
        return 0l;
    }

    return arg->results[0];
}

/**
 * @brief get parsed double value of an required identifier
 *
 * @param identifier regested identifier
 *
 * @return parsed double value
 */
double
ArgParser::getFloatValue(const std::string& identifier)
{
    std::vector<double> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return 0.0;
    }

    // check argument-type
    if (arg->type != ArgDef::FLOAT_TYPE) {
        return 0.0;
    }

    // check result not empty
    if (arg->results.size() == 0) {
        return 0.0;
    }

    return arg->results[0];
}

/**
 * @brief get parsed bool value of an required identifier
 *
 * @param identifier regested identifier
 *
 * @return parsed bool value
 */
bool
ArgParser::getBoolValue(const std::string& identifier)
{
    std::vector<bool> result;

    // get registered argument
    ArgParser::ArgDef* arg = getArgument(identifier);
    if (arg == nullptr) {
        return false;
    }

    // check argument-type
    if (arg->type != ArgDef::BOOL_TYPE) {
        return false;
    }

    // check result not empty
    if (arg->results.size() == 0) {
        return false;
    }

    return arg->results[0];
}

/**
 * @brief convert type-value into string
 *
 * @param type type-value
 *
 * @return converted string
 */
const std::string
ArgParser::convertType(ArgDef::ArgType type)
{
    if (type == ArgDef::STRING_TYPE) {
        return "string";
    }
    if (type == ArgDef::INT_TYPE) {
        return "number";
    }
    if (type == ArgDef::FLOAT_TYPE) {
        return "floating point";
    }
    if (type == ArgDef::BOOL_TYPE) {
        return "boolean";
    }

    return "";
}

/**
 * @brief print arguments on cli
 */
void
ArgParser::print(const std::string& commandName)
{
    std::string commandString = commandName + " [options]";

    // prepare table for arguments with flags
    TableItem withFlags;
    withFlags.addColumn("long");
    withFlags.addColumn("short");
    withFlags.addColumn("type");
    withFlags.addColumn("is required");
    withFlags.addColumn("help-text");

    for (uint32_t i = 0; i < m_argumentList.size(); i++) {
        if (m_argumentList.at(i).withoutFlag == false) {
            // get type
            const std::string type = convertType(m_argumentList.at(i).type);

            // required flag
            std::string required = "";
            if (m_argumentList.at(i).isRequired) {
                required = "x";
                commandString += " " + m_argumentList.at(i).longIdentifier + " ...";
            }

            // set row of table
            withFlags.addRow(std::vector<std::string>{m_argumentList.at(i).longIdentifier,
                                                      m_argumentList.at(i).shortIdentifier,
                                                      type,
                                                      required,
                                                      m_argumentList.at(i).helpText});
        }
    }

    // prepare table for arguments without flags
    TableItem withoutFlags;
    withoutFlags.addColumn("name");
    withoutFlags.addColumn("type");
    withoutFlags.addColumn("text");

    for (uint32_t i = 0; i < m_argumentList.size(); i++) {
        if (m_argumentList.at(i).withoutFlag == true) {
            // get type
            const std::string type = convertType(m_argumentList.at(i).type);

            // set row of table
            withoutFlags.addRow(
                std::vector<std::string>{"<" + m_argumentList.at(i).longIdentifier + ">",
                                         type,
                                         m_argumentList.at(i).helpText});

            commandString += " <" + m_argumentList.at(i).longIdentifier + ">";
        }
    }

    std::cout << "command: " << commandString << std::endl;
    std::cout << std::endl;
    std::cout << "Options:" << std::endl;
    std::cout << withFlags.toString(200) << std::endl;
    std::cout << "Required:" << std::endl;
    std::cout << withoutFlags.toString(200) << std::endl;

    exit(0);
}

/**
 * @brief get a registered argument
 *
 * @param identifier identifier of the argument without "--" or "-"
 *
 * @return nullptr, if identifier is unknown, else pointer to the registered argument
 */
ArgParser::ArgDef*
ArgParser::getArgument(const std::string& identifier)
{
    for (uint32_t i = 0; i < m_argumentList.size(); i++) {
        if (m_argumentList.at(i).longIdentifier == identifier
            || m_argumentList.at(i).shortIdentifier == identifier
            || m_argumentList.at(i).longIdentifier == "--" + identifier
            || m_argumentList.at(i).shortIdentifier == "-" + identifier)
        {
            return &m_argumentList.at(i);
        }
    }

    return nullptr;
}

}  // namespace Hanami
