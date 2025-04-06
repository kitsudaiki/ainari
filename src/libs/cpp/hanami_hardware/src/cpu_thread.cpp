/**
 * @file        cpu_thread.cpp
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

#include <hanami_cpu/cpu.h>
#include <hanami_hardware/cpu_core.h>
#include <hanami_hardware/cpu_package.h>
#include <hanami_hardware/cpu_thread.h>
#include <hanami_hardware/host.h>

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param threadId id of the physical cpu thread, which belongs to this thread
 */
CpuThread::CpuThread(const uint32_t threadId) : threadId(threadId) {}

/**
 * @brief destructor
 */
CpuThread::~CpuThread() {}

/**
 * @brief initialize the new thread-object and add the thread to the topological tree of objects
 *
 * @param host pointer to the host-object at the top
 *
 * @return true, if successful, else false
 */
bool
CpuThread::initThread(Host* host)
{
    ErrorContainer error;

    // min-speed
    if (getMinimumSpeed(minSpeed, threadId, error) == false) {
        LOG_WARNING(error.toString());
        error.reset();
        m_speedInitialized = false;
    }

    // max-speed
    if (getMaximumSpeed(maxSpeed, threadId, error) == false) {
        LOG_WARNING(error.toString());
        error.reset();
        m_speedInitialized = false;
    }

    // current-min-speed
    if (getCurrentMinimumSpeed(currentMinSpeed, threadId, error) == false) {
        LOG_WARNING(error.toString());
        error.reset();
        m_speedInitialized = false;
    }

    // current-max-speed
    if (getCurrentMaximumSpeed(currentMaxSpeed, threadId, error) == false) {
        LOG_WARNING(error.toString());
        error.reset();
        m_speedInitialized = false;
    }

    // core-id
    uint64_t coreId = 0;
    if (getCpuCoreId(coreId, threadId, error) == false) {
        LOG_ERROR(error);
        return false;
    }

    // package-id
    uint64_t packageId = 0;
    if (getCpuPackageId(packageId, threadId, error) == false) {
        LOG_ERROR(error);
        return false;
    }

    m_powerInitialized = false;

    // add thread to the topological overview
    CpuPackage* package = host->addPackage(packageId);
    CpuCore* core = package->addCore(coreId);
    core->addCpuThread(this);

    return true;
}

/**
 * @brief get current speed of the core
 *
 * @return -1, if reading the speed failed, else speed of the core
 */
uint64_t
CpuThread::getCurrentThreadSpeed() const
{
    if (m_speedInitialized == false) {
        return 0;
    }

    ErrorContainer error;

    uint64_t speed = 0;
    if (getCurrentSpeed(speed, threadId, error) == false) {
        LOG_ERROR(error);
        return 0;
    }

    return speed;
}

/**
 * @brief get maximum thermal spec of the package
 *
 * @return 0.0 if RAPL is not initialized, else thermal spec of the cpu-package
 */
double
CpuThread::getThermalSpec() const
{
    if (m_powerInitialized == false) {
        return 0.0;
    }

    return 0.0f;
}

/**
 * @brief get current total power consumption of the cpu-package since the last check
 *
 * @return 0.0 if RAPL is not initialized, else current total power consumption of the cpu-package
 */
double
CpuThread::getTotalPackagePower()
{
    if (m_powerInitialized == false) {
        return 0.0;
    }

    return 0.0f;
}

/**
 * @brief get information of the thread as json-formated string
 *
 * @return json-formated string with the information
 */
const std::string
CpuThread::toJsonString()
{
    std::string jsonString = "{";
    jsonString.append("\"id\":" + std::to_string(threadId));
    jsonString.append("}");

    return jsonString;
}

/**
 * @brief get information of the thread as json-like item-tree

 * @return json-like item-tree with the information
 */
json
CpuThread::toJson()
{
    json result = json::object();
    result["id"] = (long)threadId;
    return result;
}

}  // namespace Hanami
