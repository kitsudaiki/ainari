/**
 * @file       process_execution.cpp
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

#include <hanami_common/functions/string_functions.h>
#include <hanami_common/process_execution.h>

namespace Hanami
{

/**
 * @brief run a programm synchrone
 *
 * @param blossomItem blossom-item, where the results of the run should be written back
 * @param command command to execute as shell-command
 *
 * @return result with all information like exit-status and std-output
 */
ProcessResult
runSyncProcess(std::string command)
{
    std::vector<std::string> args;
    args.push_back("-c");
    // replaceSubstring(command, "\"", "\\\"");
    args.push_back("\"" + command + "\"");

    return runSyncProcess(std::string("/bin/sh"), args);
}

/**
 * @brief run a programm synchrone
 *
 * @param blossomItem blossom-item, where the results of the run should be written back
 * @param programm path to the programm to execute
 * @param args list of arguments
 *
 * @return result with all information like exit-status and std-output
 */
ProcessResult
runSyncProcess(const std::string& programm, const std::vector<std::string>& args)
{
    ProcessResult result;

    // prepare command
    std::string call = programm;
    for (uint32_t i = 0; i < args.size(); i++) {
        call += " " + args[i];
    }
    call.append(" 2>&1");

    // prepare buffer
    FILE* stream = nullptr;
    const uint32_t max_buffer = 256;
    char buffer[max_buffer];

    // start execution
    stream = popen(call.c_str(), "r");
    if (stream) {
        while (!feof(stream)) {
            if (fgets(buffer, max_buffer, stream) != nullptr) {
                result.processOutput.append(buffer);
            }
        }
        result.exitStatus = pclose(stream);
    }
    else {
        result.errorMessage = "can not execute programm: " + programm;
        result.success = false;

        return result;
    }

    // check exit-status of the external process
    if (result.exitStatus != 0) {
        result.success = false;
    }
    else {
        result.success = true;
    }

    return result;
}

}  // namespace Hanami
