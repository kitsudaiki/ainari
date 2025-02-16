/**
 * @file        logger.h
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
#ifndef HANAMI_LOGGER_H
#define HANAMI_LOGGER_H

#include <hanami_common/items/table_item.h>

#include <ctime>
#include <fstream>
#include <mutex>
#include <nlohmann/json.hpp>
#include <string>

using json = nlohmann::json;

#define LOG_DEBUG Hanami::LOG_debug
#define LOG_INFO Hanami::LOG_info
#define LOG_WARNING Hanami::LOG_warning
#define LOG_ERROR Hanami::LOG_error

#ifndef ALL_WHITE_OUTPUT
#define YELLOW_COLOR "\033[1;33m"
#define WHITE_COLOR "\033[0m"
#define GREEN_COLOR "\033[1;32m"
#define RED_COLOR "\033[1;31m"
#define BLUE_COLOR "\033[1;34m"
#define PINK_COLOR "\033[1;95m"
#else
#define YELLOW_COLOR "\033[0m"
#define WHITE_COLOR "\033[0m"
#define GREEN_COLOR "\033[0m"
#define RED_COLOR "\033[0m"
#define BLUE_COLOR "\033[0m"
#define PINK_COLOR "\033[0m"
#endif

namespace Hanami
{

struct ErrorContainer {
    bool _alreadyPrinted = false;
    std::vector<std::string> _errorMessages;
    std::vector<std::string> _possibleSolution;

    void addMessage(const std::string& errorMessage)
    {
        _errorMessages.push_back(errorMessage);
        _alreadyPrinted = false;
    }

    void addSolution(const std::string& possibleSolution)
    {
        _possibleSolution.push_back(possibleSolution);
    }

    void reset()
    {
        _errorMessages.clear();
        _possibleSolution.clear();
        _alreadyPrinted = false;
    }

    const std::string toString()
    {
        TableItem output;
        output.addColumn("key");
        output.addColumn("value");

        // add error-messages
        for (int32_t i = _errorMessages.size() - 1; i >= 0; i--) {
            output.addRowVec({"Error-Message Nr. " + std::to_string(i), _errorMessages.at(i)});
        }

        // build string with possible solutions
        std::string solutions = "";
        for (uint32_t i = 0; i < _possibleSolution.size(); i++) {
            if (i != 0) {
                solutions += "\n-----\n";
            }
            solutions += _possibleSolution.at(i);
        }

        // add possible solutions
        if (solutions.size() > 1) {
            output.addRowVec({"Possible Solution", solutions});
        }

        return output.toString(200, true);
    }
};

bool initFileLogger(const std::string& directoryPath,
                    const std::string& baseFileName,
                    const bool debugLog = false);
bool initConsoleLogger(const bool debugLog = false);
bool setDebugFlag(const bool debugLog);

bool LOG_debug(const std::string& message);
bool LOG_warning(const std::string& message);
bool LOG_error(ErrorContainer& container,
               const std::string& userId = "",
               const std::string& values = "");
bool LOG_info(const std::string& message, const std::string& color = WHITE_COLOR);

void closeLogFile();

void setErrorLogCallback(void (*handleError)(const std::string&,
                                             const std::string&,
                                             const std::string&));

//==================================================================================================

class Logger
{
   public:
    Logger();
    ~Logger();

    bool initFileLogger(const std::string& directoryPath,
                        const std::string& baseFileName,
                        const bool debugLog);
    bool initConsoleLogger(const bool debugLog);
    bool setDebugFlag(const bool debugLog);
    void setErrorLogCallback(void (*handleError)(const std::string&,
                                                 const std::string&,
                                                 const std::string&));

    void closeLogFile();

    bool logData(const std::string& message,
                 const std::string& preTag,
                 const std::string& color,
                 const bool debug = false);

    std::string m_filePath = "";
    bool m_debugLog = false;
    void (*m_handleError)(const std::string&, const std::string&, const std::string&);

    static Hanami::Logger* m_logger;

   private:
    bool m_enableConsoleLog = false;
    bool m_consoleDebugLog = false;

    bool m_enableFileLog = false;
    bool m_fileDebugLog = false;
    std::string m_directoryPath = "";
    std::string m_baseFileName = "";

    std::mutex m_lock;
    std::ofstream m_outputFile;
};

}  // namespace Hanami

#endif  // HANAMI_LOGGER_H
