/**
 * @file        cpu_package.cpp
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

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param packageId id of the package
 */
CpuPackage::CpuPackage(const uint32_t packageId) : packageId(packageId) {}

/**
 * @brief destructor
 */
CpuPackage::~CpuPackage()
{
    for (uint32_t i = 0; i < cpuCores.size(); i++) {
        delete cpuCores[i];
    }

    cpuCores.clear();
}

/**
 * @brief get a core by id
 *
 * @param coreId id of the requested core
 *
 * @return nullptr, if there is no core with the id, else pointer to the core
 */
CpuCore*
CpuPackage::getCore(const uint32_t coreId) const
{
    for (CpuCore* core : cpuCores) {
        if (core->coreId == coreId) {
            return core;
        }
    }

    return nullptr;
}

/**
 * @brief add a new core to the core
 *
 * @param coreId id of the core
 *
 * @return pointer to the core in list, if id already exist, else pointer to the new
 *         created core-object
 */
CpuCore*
CpuPackage::addCore(const uint32_t coreId)
{
    CpuCore* core = getCore(coreId);

    if (core == nullptr) {
        core = new CpuCore(coreId);
        cpuCores.push_back(core);
    }

    return core;
}

/**
 * @brief get maximum thermal spec of the package
 *
 * @return 0.0 if RAPL is not initialized, else thermal spec of the cpu-package
 */
double
CpuPackage::getThermalSpec() const
{
    if (cpuCores.size() > 0) {
        return cpuCores.at(0)->getThermalSpec();
    }

    return 0.0;
}

/**
 * @brief get current total power consumption of the cpu-package since the last check
 *
 * @return 0.0 if RAPL is not initialized, else current total power consumption of the cpu-package
 */
double
CpuPackage::getTotalPackagePower()
{
    if (cpuCores.size() > 0) {
        return cpuCores.at(0)->getTotalPackagePower();
    }

    return 0.0;
}

/**
 * @brief get information of the package as json-formated string
 *
 * @return json-formated string with the information
 */
const std::string
CpuPackage::toJsonString()
{
    // convert package-information
    std::string jsonString = "{";
    jsonString.append("\"id\":" + std::to_string(packageId));
    jsonString.append(",\"thermal_spec\":" + std::to_string(getThermalSpec()));
    jsonString.append(",\"power\":" + std::to_string(getTotalPackagePower()));
    jsonString.append(",\"cores\":[");

    // convert cores
    for (uint32_t i = 0; i < cpuCores.size(); i++) {
        if (i > 0) {
            jsonString.append(",");
        }
        jsonString.append(cpuCores.at(i)->toJsonString());
    }
    jsonString.append("]}");

    return jsonString;
}

/**
 * @brief get information of the package as json-like item-tree

 * @return json-like item-tree with the information
 */
json
CpuPackage::toJson()
{
    // convert package-information
    json result = json::object();
    result["id"] = (long)packageId;
    result["thermal_spec"] = getThermalSpec();
    result["power"] = getTotalPackagePower();

    // convert cores
    json cores = json::array();
    for (uint32_t i = 0; i < cpuCores.size(); i++) {
        cores.push_back(cpuCores.at(i)->toJson());
    }
    result["cores"] = cores;

    return result;
}

}  // namespace Hanami
