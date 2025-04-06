/**
 * @file    logger.cpp
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
#include <hanami_common/functions/time_functions.h>
#include <hanami_common/logger.h>

#include <iostream>

namespace Hanami
{

Hanami::Logger* Logger::m_logger = new Hanami::Logger();

/**
 * @brief initialize file logger
 *
 * @param directoryPath directory path where the log should be written
 * @param baseFileName base name of the log-file
 * @param debugLog true to enable debug-output
 *
 * @return false, if initializing failed, else true
 */
bool
initFileLogger(const std::string& directoryPath,
               const std::string& baseFileName,
               const bool debugLog)
{
    return Logger::m_logger->initFileLogger(directoryPath, baseFileName, debugLog);
}

/**
 * @brief initialize console logger
 *
 * @param debugLog true to enable debug-output
 *
 * @return always true
 */
bool
initConsoleLogger(const bool debugLog)
{
    return Logger::m_logger->initConsoleLogger(debugLog);
}

/**
 * @brief set debug-flag after the logger was already created
 *
 * @param debugLog new debug-flag
 *
 * @return false, if logger is not initialized, else true
 */
bool
setDebugFlag(const bool debugLog)
{
    return Logger::m_logger->setDebugFlag(debugLog);
}

/**
 * @brief default-error-callback, which is triggered by each LOG_ERROR. There is a default required
 *        to be set to avoid seg-faults.
 */
void
defaultErrorCallback(const std::string&, const std::string&, const std::string&)
{
}

/**
 * @brief write debug-message to logfile
 */
bool
LOG_debug(const std::string& message)
{
    return Logger::m_logger->logData(message, "DEBUG", BLUE_COLOR, true);
}

/**
 * @brief write warnign-message to logfile
 */
bool
LOG_warning(const std::string& message)
{
    return Logger::m_logger->logData(message, "WARNING", YELLOW_COLOR);
}

/**
 * @brief write error-message to logfile
 */
bool
LOG_error(ErrorContainer& container, const std::string& userId, const std::string& values)
{
    if (container._alreadyPrinted) {
        return true;
    }

    const std::string errorMessage = container.toString();
    const bool ret = Logger::m_logger->logData(errorMessage, "ERROR", RED_COLOR);
    Logger::m_logger->m_handleError(errorMessage, userId, values);
    if (ret) {
        container._alreadyPrinted = true;
    }

    return ret;
}

/**
 * @brief write info-message to logfile
 */
bool
LOG_info(const std::string& message, const std::string& color)
{
    return Hanami::Logger::m_logger->logData(message, "INFO", color);
}

/**
 * @brief close log-file, if one exist
 */
void
closeLogFile()
{
    Logger::m_logger->closeLogFile();
}

//==================================================================================================

/**
 * @brief constructor
 */
Logger::Logger() { setErrorLogCallback(defaultErrorCallback); }

/**
 * @brief destructor
 */
Logger::~Logger() { closeLogFile(); }

/**
 * @brief initialize file logger
 *
 * @param directoryPath directory path where the log should be written
 * @param baseFileName base name of the log-file
 * @param debugLog true to enable debug-output
 *
 * @return false, if initializing failed, else true
 */
bool
Logger::initFileLogger(const std::string& directoryPath,
                       const std::string& baseFileName,
                       const bool debugLog)
{
    std::lock_guard<std::mutex> guard(m_lock);

    m_directoryPath = directoryPath;
    m_baseFileName = baseFileName;
    m_fileDebugLog = debugLog;

    // check if already init
    if (m_enableFileLog) {
        std::cout << "ERROR: file logger is already initialized." << std::endl;
        return false;
    }

    // check if exist
    if (std::filesystem::exists(m_directoryPath) == false) {
        std::cout << "ERROR: failed to initialize logger, because the path \"" << m_directoryPath
                  << "\" does not exist." << std::endl;
        return false;
    }

    // check for directory
    if (std::filesystem::is_directory(m_directoryPath) == false) {
        std::cout << "ERROR: failed to initialize logger, because the path \"" << m_directoryPath
                  << "\" is not an directory." << std::endl;
        return false;
    }

    // create new logger-file
    m_filePath = m_directoryPath + "/" + m_baseFileName + ".log";
    m_outputFile.open(m_filePath, std::ios_base::app);
    const bool ret = m_outputFile.is_open();
    m_enableFileLog = ret;
    if (ret == false) {
        std::cout << "ERROR: can not create or open log-file-path: \"" << m_filePath << "\""
                  << std::endl;
    }

    return ret;
}

/**
 * @brief initialize console logger
 *
 * @param debugLog true to enable debug-output
 *
 * @return always true
 */
bool
Logger::initConsoleLogger(const bool debugLog)
{
    m_enableConsoleLog = true;
    m_consoleDebugLog = debugLog;

    return true;
}

/**
 * @brief set debug-flag after the logger was already created
 *
 * @param debugLog new debug-flag
 *
 * @return always true
 */
bool
Logger::setDebugFlag(const bool debugLog)
{
    m_consoleDebugLog = debugLog;
    m_fileDebugLog = debugLog;

    return true;
}

/**
 * @brief set callback for error-messages
 */
void
Logger::setErrorLogCallback(void (*handleError)(const std::string&,
                                                const std::string&,
                                                const std::string&))
{
    m_handleError = handleError;
}

/**
 * @brief close log-file if file-logging was initialized
 */
void
Logger::closeLogFile()
{
    std::lock_guard<std::mutex> guard(m_lock);

    if (m_enableFileLog) {
        m_outputFile.close();
    }
}

/**
 * @brief write message to logfile
 */
bool
Logger::logData(const std::string& message,
                const std::string& preTag,
                const std::string& color,
                const bool debug)
{
    std::lock_guard<std::mutex> guard(m_lock);

    // write to terminal
    if (m_enableConsoleLog) {
        if (debug && m_consoleDebugLog == false) {
            return false;
        }

        if (preTag == "INFO") {
            std::cout << color << message << WHITE_COLOR << std::endl;
        }
        else {
            std::cout << color << preTag << ": ";
            // special handline for tables
            if (message.size() > 0 && message.at(0) == '+') {
                std::cout << "\n";
            }
            std::cout << message << WHITE_COLOR << std::endl;
        }
    }

    // build and write new line
    if (m_enableFileLog) {
        if (debug && m_fileDebugLog == false) {
            return false;
        }

        std::string line = getDatetime() + " " + preTag + ": ";
        // special handline for tables
        if (message.size() > 0 && message.at(0) == '+') {
            line += "\n";
        }
        line += message + "\n";

        m_outputFile << line;
        m_outputFile.flush();
    }

    return true;
}

/**
 * @brief set callback for error-messages
 */
void
setErrorLogCallback(void (*handleError)(const std::string&, const std::string&, const std::string&))
{
    Logger::m_logger->setErrorLogCallback(handleError);
}

}  // namespace Hanami
