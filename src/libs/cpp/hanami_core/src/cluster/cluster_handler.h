/**
 * @file        cluster_handler.h
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

#ifndef HANAMI_CLUSTERHANDLER_H
#define HANAMI_CLUSTERHANDLER_H

#include <map>
#include <string>

class Cluster;

class ClusterHandler
{
   public:
    static ClusterHandler* getInstance()
    {
        if (instance == nullptr) {
            instance = new ClusterHandler();
        }
        return instance;
    }

    bool addCluster(const std::string uuid, Cluster* newCluster);
    bool removeCluster(const std::string uuid);
    Cluster* getCluster(const std::string uuid);

   private:
    ClusterHandler();
    static ClusterHandler* instance;

    std::map<std::string, Cluster*> m_allCluster;
};

#endif  // HANAMI_CLUSTERHANDLER_H
