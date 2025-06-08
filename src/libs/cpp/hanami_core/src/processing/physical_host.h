/**
 * @file        physical_host.h
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

#ifndef PHYSICALHOST_H
#define PHYSICALHOST_H

#include <nlohmann/json.hpp>
#include <string>
#include <vector>

class CudaHost;
class CpuHost;
class LogicalHost;
using json = nlohmann::json;

class PhysicalHost
{
   public:
    PhysicalHost(const uint64_t maxMemoryUsage);

    bool init(const uint64_t numberOfThreads, std::string& error);

    LogicalHost* getFirstHost() const;
    LogicalHost* getHost(const std::string& uuid) const;
    json getAllHostsAsJson();

   private:
    const uint64_t m_maxMemoryUsage;
    std::vector<CudaHost*> m_cudaHosts;
    std::vector<CpuHost*> m_cpuHosts;
};

#endif  // PHYSICALHOST_H
