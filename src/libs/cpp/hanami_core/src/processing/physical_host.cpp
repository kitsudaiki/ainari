/**
 * @file        physical_host.cpp
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

#include "physical_host.h"

#include <processing/cpu/cpu_host.h>
#include <processing/cuda/cuda_functions.h>
#include <processing/cuda/cuda_host.h>
#include <src/hardware/host.h>

/**
 * @brief PhysicalHost::PhysicalHost
 * @param maxMemoryUsage
 */
PhysicalHost::PhysicalHost(const float maxMemoryUsage) : m_maxMemoryUsage(maxMemoryUsage) {}

/**
 * @brief initialize all cpu's ang cuda gpu's of the physical host by creating a logical-host for
 *        each of them and giving them a local device-id
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
PhysicalHost::init(Hanami::ErrorContainer& error)
{
    LOG_INFO("Please wait a few seconds, until hardware-resources are initialized...");

    // identify and init cuda gpu's
    // IMPORTANT: these are initialized first, because they also need memory on the host
    // TODO:enable gpu init here again
    // related issue: https://github.com/kitsudaiki/Hanami/issues/325
    /*std::vector<GpuInfo> gpuInfos;
    getGpuInfos(gpuInfos);
    for (const GpuInfo& gpuInfo : gpuInfos) {
        std::cout << gpuInfo.toJson().dump(4) << std::endl;
        CudaHost* newHost = new CudaHost(gpuInfo.deviceId, gpuInfo);
        m_cudaHosts.push_back(newHost);
    }*/

    // get information of all available cpus and their threads
    if (Hanami::Host::getInstance()->initHost(error) == false) {
        return false;
    }

    // identify and init cpu's
    const uint32_t numberOfSockets = Hanami::Host::getInstance()->cpuPackages.size();
    for (uint32_t i = 0; i < numberOfSockets; i++) {
        CpuHost* newHost = new CpuHost(i, m_maxMemoryUsage);
        // this host ist not run as thead, because it has its worker-threads
        m_cpuHosts.push_back(newHost);
    }

    LOG_INFO("Initialized " + std::to_string(m_cpuHosts.size()) + " CPU-sockets");
    LOG_INFO("Initialized " + std::to_string(m_cudaHosts.size()) + " CUDA-GPUs");

    return true;
}

/**
 * @brief give first logical cpu-host
 *
 * @return pointer if at least one cpu-host exist, else nullptr
 */
LogicalHost*
PhysicalHost::getFirstHost() const
{
    if (m_cpuHosts.size() == 0) {
        return nullptr;
    }

    return m_cpuHosts.at(0);
}

/**
 * @brief get a logical host by uuid
 *
 * @param uuid uuid of the host
 *
 * @return pointer if uuid was found, else nullptr
 */
LogicalHost*
PhysicalHost::getHost(const std::string& uuid) const
{
    // check cpu
    for (LogicalHost* host : m_cpuHosts) {
        if (host->getUuid() == uuid) {
            return host;
        }
    }

    // check cuda gpu
    for (LogicalHost* host : m_cudaHosts) {
        if (host->getUuid() == uuid) {
            return host;
        }
    }

    return nullptr;
}

/**
 * @brief convert all hosts into a json for api-output
 *
 * @return json with information of the hosts
 */
json
PhysicalHost::getAllHostsAsJson()
{
    json body = json::array();
    for (LogicalHost* host : m_cpuHosts) {
        json line = json::array();
        line.push_back(host->getUuid());
        line.push_back("cpu");
        body.push_back(line);
    }
    for (LogicalHost* host : m_cudaHosts) {
        json line = json::array();
        line.push_back(host->getUuid());
        line.push_back("cuda");
        body.push_back(line);
    }

    return body;
}
