/**
 * @file        task.h
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

#ifndef HANAMI_TASK_H
#define HANAMI_TASK_H

#include <cluster/objects.h>
#include <io/data_set/dataset_file_io.h>

#include <chrono>
#include <variant>

enum TaskType {
    NO_TASK = 0,
    TRAIN_TASK = 1,
    REQUEST_TASK = 2,
};

enum TaskState {
    UNDEFINED_TASK_STATE = 0,
    QUEUED_TASK_STATE = 1,
    ACTIVE_TASK_STATE = 2,
    ABORTED_TASK_STATE = 3,
    FINISHED_TASK_STATE = 4,
};

struct TaskProgress {
    TaskState state = UNDEFINED_TASK_STATE;
    uint64_t totalNumberOfCycles = 0;
    uint64_t currentCyle = 0;
    std::chrono::high_resolution_clock::time_point queuedTimeStamp;
    std::chrono::high_resolution_clock::time_point startActiveTimeStamp;
    std::chrono::high_resolution_clock::time_point endActiveTimeStamp;
    uint64_t estimatedRemaningTime = 0;
};

struct TrainInfo {
    std::map<std::string, DataSetFileHandle> inputs;
    std::map<std::string, DataSetFileHandle> outputs;
    uint64_t numberOfCycles = 0;
    uint64_t currentCycle = 0;
    uint64_t timeLength = 1;

    TrainInfo() {}

    TrainInfo(TrainInfo&& otherObf)
    {
        inputs = std::move(otherObf.inputs);
        outputs = std::move(otherObf.outputs);
        numberOfCycles = otherObf.numberOfCycles;
        currentCycle = otherObf.currentCycle;
    }

    TrainInfo& operator=(TrainInfo&& otherObf)
    {
        inputs = std::move(otherObf.inputs);
        outputs = std::move(otherObf.outputs);
        numberOfCycles = otherObf.numberOfCycles;
        currentCycle = otherObf.currentCycle;

        return *this;
    }
};

struct RequestInfo {
    std::map<std::string, DataSetFileHandle> inputs;
    std::map<std::string, DataSetFileHandle> results;
    uint64_t numberOfCycles = 0;
    uint64_t currentCycle = 0;
    uint64_t timeLength = 1;

    RequestInfo() {}

    RequestInfo(RequestInfo&& otherObf)
    {
        inputs = std::move(otherObf.inputs);
        results = std::move(otherObf.results);
        numberOfCycles = otherObf.numberOfCycles;
        currentCycle = otherObf.currentCycle;
    }

    RequestInfo& operator=(RequestInfo&& otherObf)
    {
        inputs = std::move(otherObf.inputs);
        results = std::move(otherObf.results);
        numberOfCycles = otherObf.numberOfCycles;
        currentCycle = otherObf.currentCycle;

        return *this;
    }
};

struct Task {
    UUID uuid;
    TaskType type = NO_TASK;
    std::string name = "";

    // progress
    TaskProgress progress;

    std::variant<TrainInfo, RequestInfo> info;

    Task()
    {
        // TODO
        // uuid = generateUuid();
        progress.state = QUEUED_TASK_STATE;
        progress.queuedTimeStamp = std::chrono::system_clock::now();
    }

    Task(Task&& otherObf)
    {
        // TODO
        // uuid = otherObf.uuid;
        type = otherObf.type;
        name = std::move(otherObf.name);

        // progress
        progress = otherObf.progress;

        info = std::move(info);
    }

    Task& operator=(Task&& otherObf)
    {
        uuid = otherObf.uuid;
        type = otherObf.type;
        name = std::move(otherObf.name);

        // progress
        progress = otherObf.progress;

        info = std::move(info);

        return *this;
    }
};

#endif  // HANAMI_TASK_H
