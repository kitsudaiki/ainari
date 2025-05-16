/**
 * @file        cluster.h
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

#ifndef HANAMI_CLUSTER_LINK_H
#define HANAMI_CLUSTER_LINK_H

#include <stdint.h>

#include <string>

class Cluster;

class ClusterLink
{
   public:
    ClusterLink(Cluster* cluster);
    ~ClusterLink();

    int createCheckpoint(const std::string& targetFilePath);

    bool fillInput(const std::string& hexagonName,
                   const float* input,
                   const uint64_t numberOfInputs);
    bool fillExpected(const std::string& hexagonName,
                      const float* output,
                      const uint64_t numberOfOutputs);
    bool getOutput(const std::string& hexagonName, float* output, const uint64_t numberOfOutputs);
    uint64_t getSize(const std::string& hexagonName);

    void doTrain();
    void doRequest();

   private:
    Cluster* m_cluster = nullptr;
};

#endif  // HANAMI_CLUSTER_LINK_H
