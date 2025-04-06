/**
 * @file        host.h
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

#ifndef HANAMI_HARDWARE_HOST_H
#define HANAMI_HARDWARE_HOST_H

#include <hanami_common/logger.h>
#include <unistd.h>

#include <string>
#include <vector>

namespace Hanami
{
class CpuPackage;

class Host
{
   public:
    static Host* getInstance()
    {
        if (instance == nullptr) {
            instance = new Host();
        }
        return instance;
    }

    ~Host();

    bool initHost(ErrorContainer& error);

    std::string hostName;
    bool hasHyperThrading;
    std::vector<CpuPackage*> cpuPackages;

    CpuPackage* getPackage(const uint32_t pos) const;
    CpuPackage* addPackage(const uint32_t packageId);

    double getTotalTemperature(ErrorContainer& error);

    const std::string toJsonString() const;
    json toJson() const;

   private:
    Host();
    static Host* instance;

    bool readHostName(ErrorContainer& error);
    bool initCpuCoresAndThreads(ErrorContainer& error);
};

}  // namespace Hanami

#endif  // HANAMI_HARDWARE_HOST_H
