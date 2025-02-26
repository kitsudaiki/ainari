/**
 * @file        cpu_core.h
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

#ifndef HANAMI_HARDWARE_CPUCORE_H
#define HANAMI_HARDWARE_CPUCORE_H

#include <nlohmann/json.hpp>
#include <string>
#include <vector>

using json = nlohmann::json;

namespace Hanami
{
class CpuThread;

class CpuCore
{
   public:
    CpuCore(const uint32_t coreId);
    ~CpuCore();

    const uint32_t coreId;

    CpuThread* getThread(const uint32_t threadId) const;
    CpuThread* addThread(const uint32_t threadId);
    void addCpuThread(CpuThread* thread);

    double getThermalSpec() const;
    double getTotalPackagePower();

    const std::string toJsonString();
    json toJson();

    std::vector<CpuThread*> cpuThreads;
};

}  // namespace Hanami

#endif  // HANAMI_HARDWARE_CPUCORE_H
