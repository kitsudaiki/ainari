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

#include <hanami_common/logger.h>
#include <hanami_common/structs.h>
#include <include/hanami_core/types.h>

#include <memory>
#include <mutex>
#include <string>
#include <vector>

class PhysicalHost;

namespace Hanami
{
struct ErrorContainer;
class Host;
class GpuInterface;
}  // namespace Hanami
class DataSetFileHandle;

struct InputMeta {
    std::string name = "";
    uint32_t targetHexagonId = UNINTI_POINT_32;
};

struct OutputMeta {
    std::string name = "";
    uint32_t targetHexagonId = UNINTI_POINT_32;
    OutputType type = PLAIN_OUTPUT;
};

struct AxonMeta {
    uint32_t sourceId = UNINTI_POINT_32;
    uint32_t targetId = UNINTI_POINT_32;
};

struct ClusterMeta {
    uint32_t version = 0;
    float neuronCooldown = 1000000000.f;
    uint32_t refractoryTime = 1;
    uint32_t maxConnectionDistance = 1;

    std::vector<Hanami::Position> hexagons;
    std::vector<InputMeta> inputs;
    std::vector<OutputMeta> outputs;
    std::vector<AxonMeta> axons;
};

class HanamiCore
{
   public:
    HanamiCore();
    ~HanamiCore();

    bool init(const float maxMemoryUsage, std::string& errorMessage);

    ReturnStatus createCluster(const std::string& clusterUuid,
                               const std::string& name,
                               const ClusterMeta& parsedCluster,
                               std::string& errorMessage);
    ReturnStatus deleteCluster(const std::string& clusterUuid);
    ReturnStatus setClusterMode(const std::string& clusterUuid,
                                const std::string& mode,
                                std::string& errorMessage);

    ReturnStatus createCheckpoint(const std::string& clusterUuid,
                                  const std::string& targetFilePath,
                                  std::string& errorMessage);

    ReturnStatus createTrainTask(const std::string& clusterUuid,
                                 const std::string& taskName,
                                 const std::vector<TaskLink>& inputs,
                                 const std::vector<TaskLink>& outputs,
                                 const float timeLength,
                                 std::string& errorMessage);
    ReturnStatus createRequestTask(const std::string& clusterUuid,
                                   const std::string& taskName,
                                   const std::vector<TaskLink>& inputs,
                                   const std::vector<TaskLink>& outputs,
                                   const float timeLength,
                                   std::string& errorMessage);

    int area() const;
    int perimeter() const;
    void my_cpp_function();

    static Hanami::GpuInterface* gpuInterface;
    static HanamiCore* rootObj;
    static PhysicalHost* physicalHost;

   private:
    bool m_isInit = false;
    std::mutex m_clusterMutex;
    ReturnStatus _fillTaskIo(DataSetFileHandle& fileHandle,
                             const std::string& clusterUuid,
                             const TaskLink& taskLink,
                             Hanami::ErrorContainer& error);
};

std::unique_ptr<HanamiCore> createRootObj();

#endif  // HANAMI_HANAMI_ROOT_H
