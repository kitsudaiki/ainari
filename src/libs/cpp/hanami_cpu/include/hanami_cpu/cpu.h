/**
 * @file       cpu.h
 *
 *  @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *  @copyright  Apache License Version 2.0
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

#ifndef KITSUNEMIMI_CPU_CPU_H
#define KITSUNEMIMI_CPU_CPU_H

#include <hanami_common/logger.h>
#include <stdint.h>
#include <stdlib.h>

namespace Hanami
{

// topological
bool getNumberOfCpuPackages(uint64_t& result, ErrorContainer& error);
bool getNumberOfCpuThreads(uint64_t& result, ErrorContainer& error);
bool getCpuPackageId(uint64_t& result, const uint64_t threadId, ErrorContainer& error);
bool getCpuCoreId(uint64_t& result, const uint64_t threadId, ErrorContainer& error);
bool getCpuSiblingId(uint64_t& result, const uint64_t threadId, ErrorContainer& error);

// hyperthreading
bool isHyperthreadingEnabled(ErrorContainer& error);
bool isHyperthreadingSupported(ErrorContainer& error);
bool changeHyperthreadingState(const bool newState, ErrorContainer& error);

// speed
bool getMinimumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error);
bool getMaximumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error);
bool getCurrentSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error);

bool getCurrentMinimumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error);
bool getCurrentMaximumSpeed(uint64_t& result, const uint64_t threadId, ErrorContainer& error);

bool setMinimumSpeed(const uint64_t threadId, uint64_t newSpeed, ErrorContainer& error);
bool setMaximumSpeed(const uint64_t threadId, uint64_t newSpeed, ErrorContainer& error);
bool resetSpeed(const uint64_t threadId, ErrorContainer& error);

// temperature
bool getPkgTemperatureIds(std::vector<uint64_t>& ids, ErrorContainer& error);
double getPkgTemperature(const uint64_t pkgFileId, ErrorContainer& error);

}  // namespace Hanami

#endif  // KITSUNEMIMI_CPU_CPU_H
