/**
 * @file        hanami_core.h
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

#ifndef HANAMI_HANAMI_ROOT_H
#define HANAMI_HANAMI_ROOT_H

#include <memory>
#include <mutex>
#include <string>

#include "hanami_structs.h"

class PhysicalHost;
class ClusterLink;

namespace Hanami
{
class GpuInterface;
}  // namespace Hanami

class HanamiCore
{
   public:
    HanamiCore();
    ~HanamiCore();

    bool init(const float maxMemoryUsage, std::string& errorMessage);

    std::unique_ptr<ClusterLink> createCluster(const std::string& clusterUuid,
                                               const std::string& name,
                                               const ClusterMeta& parsedCluster,
                                               std::string& errorMessage);
    int deleteCluster(const std::string& clusterUuid);

    static Hanami::GpuInterface* gpuInterface;
    static HanamiCore* rootObj;
    static PhysicalHost* physicalHost;

   private:
    bool m_isInit = false;
    std::mutex m_clusterMutex;
};

std::unique_ptr<HanamiCore> createRootObj();

#endif  // HANAMI_HANAMI_ROOT_H
