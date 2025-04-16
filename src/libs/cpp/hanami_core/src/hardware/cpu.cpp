/**
 * @file       cpu.cpp
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

#include <src/common/functions/file_functions.h>
#include <src/common/functions/string_functions.h>
#include <src/hardware/cpu.h>

namespace Hanami
{

/**
 * @brief generic function to get file-content of a requested file
 *
 * @param filePath path to the file
 * @param error reference for error-output
 *
 * @return file-content, if available, else empty string
 */
const std::string
getInfo(const std::string& filePath, ErrorContainer& error)
{
    // open file
    std::ifstream inFile;
    inFile.open(filePath);
    if (inFile.is_open() == false) {
        error.addMessage("can not open file to read content: '" + filePath + "'");
        error.addSolution("check if you have read-permissions to the file '" + filePath + "'");
        error.addSolution("check if the file  '" + filePath + "' exist on your system");
        return "";
    }

    // get file-content
    std::stringstream strStream;
    strStream << inFile.rdbuf();
    inFile.close();

    // make content clean
    std::string content = strStream.str();
    Hanami::trim(content);

    return content;
}

/**
 * @brief get max-value of a range-info-output
 *
 * @param result reference for result-output
 * @param info string with the info to parse
 *
 * @return true, if successfull, else false
 */
bool
getRangeInfo(uint64_t& result, const std::string& info)
{
    // handle case of only one core
    if (info == "0") {
        result = 1;
        return true;
    }

    // process content
    std::vector<std::string> numberRange;
    Hanami::splitStringByDelimiter(numberRange, info, '-');
    if (numberRange.size() < 2) {
        return false;
    }

    result = std::stoi(numberRange.at(1)) + 1;
    return true;
}

/**
 * @brief write value into file
 *
 * @param filePath absolute file-path
 * @param value value to write
 * @param error reference for error-output
 *
 * @return false, if no permission to update files, else true
 */
bool
writeToFile(const std::string& filePath, const std::string& value, ErrorContainer& error)
{
    // open file
    std::ofstream outputFile;
    outputFile.open(filePath, std::ios_base::in);
    if (outputFile.is_open() == false) {
        error.addMessage("can not open file to write content: '" + filePath + "'");
        error.addSolution("check if you have write-permissions to the file '" + filePath + "'");
        error.addSolution("check if the file  '" + filePath + "' exist on your system");
        return false;
    }

    // update file
    outputFile << value;
    outputFile.flush();
    outputFile.close();

    return true;
}

/**
 * @brief write new value into a cpu-freq-file
 *
 * @param value new value to write into the file
 * @param threadId id of the thread to change
 * @param fileName target file-name to update
 * @param error reference for error-output
 *
 * @return false, if no permission to update files, else true
 */
bool
writeToFile(const uint64_t value,
            const uint64_t threadId,
            const std::string& fileName,
            ErrorContainer& error)
{
    // build target-path
    const std::string filePath
        = "/sys/devices/system/cpu/cpu" + std::to_string(threadId) + "/cpufreq/" + fileName;

    return writeToFile(filePath, std::to_string(value), error);
}

/**
 * @brief get number of cpu-sockets of the system
 *
 * @param result reference for result-output
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getNumberOfCpuPackages(uint64_t& result, ErrorContainer& error)
{
    // get info from requested file
    const std::string filePath = "/sys/devices/system/node/possible";
    const std::string info = getInfo(filePath, error);
    if (info == "") {
        error.addMessage(
            "Failed to get number of cpu-packages, "
            "because can not read file '"
            + filePath + "'");
        return false;
    }

    // process file-content
    uint64_t range = 0;
    if (getRangeInfo(range, info) == false) {
        error.addMessage(
            "Failed to get number of cpu-packages, "
            "because something seems to be broken in file '"
            + filePath + "'");
        error.addSolution("Check read-permissions for file '" + filePath + "'");
        return false;
    }

    result = range;
    return true;
}

/**
 * @brief get total number of cpu-threads of all cpu-sockets of the system
 *
 * @param result reference for result-output
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getNumberOfCpuThreads(uint64_t& result, ErrorContainer& error)
{
    // get info from requested file
    const std::string filePath = "/sys/devices/system/cpu/present";
    const std::string info = getInfo(filePath, error);
    if (info == "") {
        error.addMessage(
            "Failed to get number of cpu-threads, "
            "because can not read file '"
            + filePath + "'");
        return false;
    }

    // process file-content
    uint64_t range = 0;
    if (getRangeInfo(range, info) == false) {
        error.addMessage(
            "Failed to get number of cpu-threads, "
            "because something seems to be broken in file '"
            + filePath + "'");
        error.addSolution("Check read-permissions for file '" + filePath + "'");
        return false;
    }

    result = range;
    return true;
}

/**
 * @brief check if hyperthreading is enabled on the system
 *
 * @param error reference for error-output
 *
 * @return true, if hyperthreading is enabled, else false
 */
bool
isHyperthreadingEnabled(ErrorContainer& error)
{
    const std::string filePath = "/sys/devices/system/cpu/smt/active";
    const std::string active = getInfo(filePath, error);
    if (active == "") {
        error.addMessage("Failed to check if hyperthreading is active");
        return false;
    }

    return active == "1";
}

/**
 * @brief check if hyperthreading is suppored by the system
 *
 * @param error reference for error-output
 *
 * @return true, if supported, else false
 */
bool
isHyperthreadingSupported(ErrorContainer& error)
{
    const std::string filePath = "/sys/devices/system/cpu/smt/control";
    const std::string htState = getInfo(filePath, error);
    if (htState == "") {
        error.addMessage("Failed to check if hyperthreading is supported");
        return false;
    }

    // htState can be "on", "off" or "notsupported"
    return htState != "notsupported";
}

/**
 * @brief changeHyperthreadingState
 *
 * @param newState true to enable and false to disable hyperthreading
 * @param error reference for error-output
 *
 * @return true, if hyperthreading is suppored and changes successful, else false
 */
bool
changeHyperthreadingState(const bool newState, ErrorContainer& error)
{
    const std::string filePath = "/sys/devices/system/cpu/smt/control";
    const std::string htState = getInfo(filePath, error);
    if (htState == "") {
        error.addMessage("Failed to check if hyperthreading is supported");
        return false;
    }

    // check if hyperthreading is supported
    if (htState == "notsupported") {
        error.addMessage(
            "Failed to set new hyperthreading-state, "
            "because hyperthreading is not suppoorted by the cpu");
        error.addSolution("Buy a new cpu, which supports hyperthreading ;-) ");
        return false;
    }

    if (newState) {
        // check if hyperthreading is already active
        if (htState == "on") {
            return true;
        }

        // set new state
        if (writeToFile(filePath, "on", error) == false) {
            error.addMessage("Failed to enable hyperthreading");
            return false;
        }

        return true;
    }
    else {
        // check if hyperthreading is already disabled
        if (htState == "off") {
            return true;
        }

        // set new state
        if (writeToFile(filePath, "off", error) == false) {
            error.addMessage("Failed to disable hyperthreading");
            return false;
        }

        return true;
    }

    return true;
}

/**
 * @brief check to which cpu-socket a specific thread belongs to
 *
 * @param result reference for result-output
 * @param threadId id of the thread to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getCpuPackageId(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    // build request-path
    const std::string filePath = "/sys/devices/system/cpu/cpu" + std::to_string(threadId)
                                 + "/topology/physical_package_id";

    // get info from requested file
    const std::string info = getInfo(filePath, error);
    if (info == "") {
        error.addMessage("Failed to get package-id of the cpu-thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    result = std::stoi(info);
    return true;
}

/**
 * @brief get id of the physical core of a cpu-thread
 *
 * @param result reference for result-output
 * @param threadId id of the thread to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getCpuCoreId(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    // build request-path
    const std::string filePath
        = "/sys/devices/system/cpu/cpu" + std::to_string(threadId) + "/topology/core_id";

    // get info from requested file
    const std::string info = getInfo(filePath, error);
    if (info == "") {
        error.addMessage("Failed to get core-id of the cpu-thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    result = std::stoi(info);
    return true;
}

/**
 * @brief get thread-id of a sibling to another thread-id in case of hyper-threading
 *
 * @param result reference for result-output
 * @param threadId id of the thread create to request
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getCpuSiblingId(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    // if hyperthreading is not enabled, there are no siblings possible
    if (isHyperthreadingEnabled(error)) {
        error.addMessage("Failed to get sibling-id of the cpu-thread with id: '"
                         + std::to_string(threadId)
                         + "', because hyperthreading is not enabled or supported");
        error.addSolution("Endable hyperthrading, if supported by the system");
        return false;
    }

    // build request-path
    const std::string filePath = "/sys/devices/system/cpu/cpu" + std::to_string(threadId)
                                 + "/topology/thread_siblings_list";

    // get info from requested file
    const std::string info = getInfo(filePath, error);
    if (info == "") {
        error.addMessage("Failed to get sibling-id of the cpu-thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    // process content
    std::vector<std::string> siblings;
    Hanami::splitStringByDelimiter(siblings, info, ',');
    if (siblings.size() < 2) {
        error.addMessage("Failed to get sibling-id of the cpu-thread with id: '"
                         + std::to_string(threadId) + "'");
        error.addSolution("Check if file '" + filePath + "' has contains a comma-separated"
                          " list of thread-ids");
        return false;
    }

    // filter correct result from the output
    const uint64_t second = std::stoi(siblings.at(1));
    if (second == threadId) {
        return std::stoi(siblings.at(0));
    }

    result = second;
    return true;
}

/**
 * @brief get speed-value of a file of a thread
 *
 * @param result reference for result-output
 * @param threadId id of the thread create to request
 * @param fileName name of the file in cpufreq-directory to read
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getSpeed(uint64_t& result,
         const uint64_t threadId,
         const std::string& fileName,
         ErrorContainer& error)
{
    // build request-path
    const std::string filePath
        = "/sys/devices/system/cpu/cpu" + std::to_string(threadId) + "/cpufreq/" + fileName;

    // get info from requested file
    const std::string info = getInfo(filePath, error);
    if (info == "") {
        return false;
    }

    result = std::stol(info);
    return true;
}

/**
 * @brief get current set minimum speed of a cpu-thread
 *
 * @param result reference for result-output
 * @param threadId id of thread to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getCurrentMinimumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    uint64_t speed = 0;
    if (getSpeed(speed, threadId, "scaling_min_freq", error) == false) {
        error.addMessage("Failed to the current minimum speed of thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    result = speed;
    return true;
}

/**
 * @brief get current set maximum speed of a cpu-thread
 *
 * @param result reference for result-output
 * @param threadId id of thread to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getCurrentMaximumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    uint64_t speed = 0;
    if (getSpeed(speed, threadId, "scaling_max_freq", error) == false) {
        error.addMessage("Failed to the current maximum speed of thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    result = speed;
    return true;
}

/**
 * @brief get current speed of a cpu-thread
 *
 * @param result reference for result-output
 * @param threadId id of thread to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getCurrentSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    uint64_t speed = 0;
    if (getSpeed(speed, threadId, "scaling_cur_freq", error) == false) {
        error.addMessage("Failed to the current speed of thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    result = speed;
    return true;
}

/**
 * @brief get absolute minimum value of a thread
 *
 * @param result reference for result-output
 * @param threadId id of thread to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getMinimumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    uint64_t speed = 0;
    if (getSpeed(speed, threadId, "cpuinfo_min_freq", error) == false) {
        error.addMessage("Failed to the minimum speed of thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    result = speed;
    return true;
}

/**
 * @brief get absolute maxiumum value of a thread
 *
 * @param result reference for result-output
 * @param threadId id of thread to check
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
getMaximumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error)
{
    uint64_t speed = 0;
    if (getSpeed(speed, threadId, "cpuinfo_max_freq", error) == false) {
        error.addMessage("Failed to the maximum speed of thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    result = speed;
    return true;
}

/**
 * @brief set minimum speed value of cpu-thread
 *
 * @param threadId id of the thread to change
 * @param newSpeed new minimum speed-value to set in Hz
 * @param error reference for error-output
 *
 * @return false, if no permission to update files, else true
 */
bool
setMinimumSpeed(const uint64_t threadId, uint64_t newSpeed, ErrorContainer& error)
{
    // fix lower border
    uint64_t minSpeed = 0;
    if (getMinimumSpeed(minSpeed, threadId, error) == false) {
        return false;
    }
    if (newSpeed < minSpeed) {
        newSpeed = minSpeed;
    }

    // fix upper border
    uint64_t maxSpeed = 0;
    if (getMaximumSpeed(maxSpeed, threadId, error) == false) {
        return false;
    }
    if (newSpeed > maxSpeed) {
        newSpeed = maxSpeed;
    }

    // set new in value
    if (writeToFile(newSpeed, threadId, "scaling_min_freq", error) == false) {
        error.addMessage("Failed to set new minimum speed for thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    return true;
}

/**
 * @brief setMinimumSpeed
 *
 * @param threadId id of the thread to change
 * @param newSpeed new maximum speed-value to set in Hz
 * @param error reference for error-output
 *
 * @return false, if no permission to update files, else true
 */
bool
setMaximumSpeed(const uint64_t threadId, uint64_t newSpeed, ErrorContainer& error)
{
    // fix lower border
    uint64_t minSpeed = 0;
    if (getMinimumSpeed(minSpeed, threadId, error) == false) {
        return false;
    }
    if (newSpeed < minSpeed) {
        newSpeed = minSpeed;
    }

    // fix upper border
    uint64_t maxSpeed = 0;
    ;
    if (getMaximumSpeed(maxSpeed, threadId, error) == false) {
        return false;
    }
    if (newSpeed > maxSpeed) {
        newSpeed = maxSpeed;
    }

    // set new max value
    if (writeToFile(newSpeed, threadId, "scaling_max_freq", error) == false) {
        error.addMessage("Failed to set new maximum speed for thread with id: '"
                         + std::to_string(threadId) + "'");
        return false;
    }

    return true;
}

/**
 * @brief reset speed values to basic values
 *
 * @param threadId id of the thread to reset
 * @param error reference for error-output
 *
 * @return false, if no permission to update files, else true
 */
bool
resetSpeed(const uint64_t threadId, ErrorContainer& error)
{
    // reset max-speed
    uint64_t maxSpeed = 0;
    ;
    if (getMaximumSpeed(maxSpeed, threadId, error) == false) {
        return false;
    }
    if (setMaximumSpeed(threadId, maxSpeed, error) == false) {
        return false;
    }

    // reset min-speed
    uint64_t minSpeed = 0;
    if (getMinimumSpeed(minSpeed, threadId, error) == false) {
        return false;
    }
    if (setMinimumSpeed(threadId, minSpeed, error) == false) {
        return false;
    }

    return true;
}

/**
 * @brief get file-ids, which contains temperature-ids of the cpu
 *
 * @param ids reference for the resulting ids
 * @param error reference for error-output
 *
 * @return false if no ids were found, else true
 */
bool
getPkgTemperatureIds(std::vector<uint64_t>& ids, ErrorContainer& error)
{
    uint64_t counter = 0;
    const std::string basePath = "/sys/class/thermal/thermal_zone";

    while (true) {
        const std::string filePath = basePath + std::to_string(counter) + "/type";

        // break-rule to avoid endless-loop
        if (std::filesystem::exists(filePath) == false) {
            if (ids.size() == 0) {
                error.addMessage(
                    "No files found with relevant temperature-information "
                    "about the cpu");
                return false;
            }

            return true;
        }

        // get type-information behind the id
        const std::string content = getInfo(filePath, error);
        if (content == "") {
            return false;
        }

        // check if the id belongs to the temperature of the cpu-package
        // TODO: check if this also is correct in Multi-CPU Server
        if (content == "x86_pkg_temp") {
            ids.push_back(counter);
        }

        counter++;
    }

    return true;
}

/**
 * @brief get temperature of a cpu
 *
 * @param pkgFileId one of the ids, which where selected by the function getPkgTemperatureIds
 * @param error reference for error-output
 *
 * @return 0.0 if no temperature was found for the id,
 *             else the temperature behind the id in celsius
 */
double
getPkgTemperature(const uint64_t pkgFileId, ErrorContainer& error)
{
    const std::string filePath
        = "/sys/class/thermal/thermal_zone" + std::to_string(pkgFileId) + "/temp";

    // get type-information behind the id
    const std::string content = getInfo(filePath, error);
    if (content == "") {
        return 0.0;
    }

    // convert content into value
    const long temp = strtol(content.c_str(), NULL, 10);
    return (double)temp / 1000.0;
}

}  // namespace Hanami
