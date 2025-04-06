/**
 * @file        hanami_core.cpp
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

#include <hanami_common/files/binary_file.h>
#include <hanami_common/files/text_file.h>
#include <hanami_common/functions/file_functions.h>
#include <hanami_common/logger.h>
#include <hanami_common/statemachine.h>
#include <include/hanami_core/hanami_root.h>
#include <src/cluster/cluster.h>
#include <src/cluster/cluster_handler.h>
#include <src/cluster/cluster_init.h>
#include <src/cluster/statemachine_init.h>
#include <src/io/checkpoint/disc/checkpoint_io.h>
#include <src/processing/logical_host.h>
#include <src/processing/physical_host.h>
#include <src/thread_binder.h>

#include <iostream>

// init static variables
Hanami::GpuInterface* HanamiCore::gpuInterface = nullptr;
HanamiCore* HanamiCore::rootObj = nullptr;
PhysicalHost* HanamiCore::physicalHost = nullptr;

/**
 * @brief constructor
 */
HanamiCore::HanamiCore() { rootObj = this; }

/**
 * @brief destructor
 */
HanamiCore::~HanamiCore() { delete Hanami::Logger::m_logger; }

/**
 * @brief init root-object
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
HanamiCore::init(const float maxMemoryUsage, std::string& errorMessage)
{
    if (m_isInit) {
        return false;
    }

    srand(time(NULL));

    // inti hosts
    Hanami::ErrorContainer error;
    physicalHost = new PhysicalHost(maxMemoryUsage);
    if (physicalHost->init(error) == false) {
        delete physicalHost;
        LOG_ERROR(error);
        errorMessage = error.toString();
        return false;
    }

    // create thread-binder
    if (ThreadBinder::getInstance()->init(error) == false) {
        error.addMessage("failed to init thread-binder");
        delete physicalHost;
        LOG_ERROR(error);
        errorMessage = error.toString();
        return false;
    }
    ThreadBinder::getInstance()->startThread();

    m_isInit = true;

    return true;
}

/**
 * @brief HanamiCore::createCluster
 * @param uuid
 * @param name
 * @param clusterTemplate
 * @param errorMessage
 * @return
 */
ReturnStatus
HanamiCore::createCluster(const std::string& uuid,
                          const std::string& name,
                          const std::string& clusterTemplate,
                          std::string& errorMessage)
{
    Hanami::ErrorContainer error;
    std::lock_guard<std::mutex> guard(m_clusterMutex);
    Cluster* newCluster = nullptr;

    do {
        // check if cluster already exist
        if (ClusterHandler::getInstance()->getCluster(uuid) != nullptr) {
            error.addMessage("Cluster with UUID '" + uuid + "' already exist.");
            break;
        }

        // parse cluster-template to validate syntax
        Hanami::ClusterMeta parsedCluster;
        if (Hanami::parseCluster(&parsedCluster, clusterTemplate, error) == false) {
            error.addMessage("Uploaded template is not a valid cluster-template");
            error.addMessage(error.toString());
            break;
        }

        // create new cluster
        newCluster = new Cluster();
        if (newCluster->clusterHeader.name.setName(name) == false) {
            error.addMessage("New cluster-name '" + name
                             + "' too long, even this should be avoided by the API.");
            break;
        }

        // generate and initialize the cluster based on the cluster-templates
        if (newCluster->init(parsedCluster, uuid) == false) {
            error.addMessage("Failed to initialize cluster based on a template");
            break;
        }

        // add to cluster-handler
        if (ClusterHandler::getInstance()->addCluster(uuid, newCluster) == false) {
            error.addMessage("Failed to add cluster to cluster-handler.");
            break;
        }

        return OK;
    }
    while (true);

    // cleanup in case of a failure
    if (newCluster != nullptr) {
        delete newCluster;
    }
    errorMessage = error.toString();

    return INVALID_INPUT;
}

/**
 * @brief HanamiCore::deleteCluster
 * @param uuid
 * @return
 */
ReturnStatus
HanamiCore::deleteCluster(const std::string& uuid)
{
    std::lock_guard<std::mutex> guard(m_clusterMutex);

    // TODO: stop acitve task if necessasry
    if (ClusterHandler::getInstance()->removeCluster(uuid) == false) {
        return INVALID_INPUT;
    }

    return OK;
}

/**
 * @brief HanamiCore::setClusterMode
 * @param clusterUuid
 * @param mode
 * @param errorMessage
 * @return
 */
ReturnStatus
HanamiCore::setClusterMode(const std::string& clusterUuid,
                           const std::string& mode,
                           std::string& errorMessage)
{
    Hanami::ErrorContainer error;

    if (mode != "TASK" && mode != "DIRECT") {
        error.addMessage("Failed to set cluster-mode, because mode '" + mode + "' is unkown");
        errorMessage = error.toString();
        return INVALID_INPUT;
    }

    std::lock_guard<std::mutex> guard(m_clusterMutex);

    Cluster* cluster = ClusterHandler::getInstance()->getCluster(clusterUuid);
    if (cluster == nullptr) {
        error.addMessage("Cluster with UUID '" + clusterUuid
                         + "'not found even it exists within the database");
        errorMessage = error.toString();
        return INVALID_INPUT;
    }

    // switch mode of cluster
    if (cluster->setClusterState(mode) == false) {
        error.addMessage("Can not switch Cluster with uuid '" + clusterUuid + "' to new mode '"
                         + mode + "'");
        errorMessage = error.toString();
        return ERROR;
    }

    return OK;
}

/**
 * @brief HanamiCore::createCheckpoint
 * @param clusterUuid
 * @param targetFilePath
 * @param errorMessage
 * @return
 */
ReturnStatus
HanamiCore::createCheckpoint(const std::string& clusterUuid,
                             const std::string& targetFilePath,
                             std::string& errorMessage)
{
    Hanami::ErrorContainer error;
    std::filesystem::path filePath = targetFilePath;

    std::lock_guard<std::mutex> guard(m_clusterMutex);

    Cluster* cluster = ClusterHandler::getInstance()->getCluster(clusterUuid);
    if (cluster == nullptr) {
        error.addMessage("Cluster with UUID '" + clusterUuid
                         + "'not found even it exists within the database");
        errorMessage = error.toString();
        return INVALID_INPUT;
    }

    // cluster->stateMachine
    for (Hexagon& hexagon : cluster->hexagons) {
        hexagon.attachedHost->syncWithHost(&hexagon);
    }
    CheckpointIO m_clusterIO;
    ReturnStatus ret = m_clusterIO.writeClusterToFile(*cluster, targetFilePath, error);
    if (ret != OK) {
        return ret;
    }

    return OK;
}

/**
 * @brief HanamiCore::createTrainTask
 * @param clusterUuid
 * @param taskName
 * @param inputs
 * @param outputs
 * @param timeLength
 * @param errorMessage
 * @return
 */
ReturnStatus
HanamiCore::createTrainTask(const std::string& clusterUuid,
                            const std::string& taskName,
                            const std::vector<TaskLink>& inputs,
                            const std::vector<TaskLink>& outputs,
                            const float timeLength,
                            std::string& errorMessage)
{
    Hanami::ErrorContainer error;
    ReturnStatus ret = ERROR;
    std::lock_guard<std::mutex> guard(m_clusterMutex);

    // get cluster
    Cluster* cluster = ClusterHandler::getInstance()->getCluster(clusterUuid);
    if (cluster == nullptr) {
        error.addMessage("Cluster with UUID '" + clusterUuid + "'not found");
        return INVALID_INPUT;
    }

    // create new train-task
    Task* newTask = cluster->addNewTask();
    if (newTask == nullptr) {
        return ERROR;
    }

    newTask->name = taskName;
    newTask->type = TRAIN_TASK;
    newTask->progress.queuedTimeStamp = std::chrono::system_clock::now();
    newTask->info = TrainInfo();
    TrainInfo* info = &std::get<TrainInfo>(newTask->info);
    info->timeLength = timeLength;
    uint64_t numberOfCycles = std::numeric_limits<uint64_t>::max();

    // prepare inputs
    for (const TaskLink& link : inputs) {
        DataSetFileHandle fileHandle;
        ret = _fillTaskIo(fileHandle, clusterUuid, link, error);
        if (ret != OK) {
            return ret;
        }
        if (numberOfCycles > fileHandle.header.numberOfRows) {
            numberOfCycles = fileHandle.header.numberOfRows;
        }

        // resize number of inputs and size of io-buffer for the given data
        InputInterface* inputInterface = &cluster->inputInterfaces[link.hexagonId];
        const uint64_t numberOfColumns
            = fileHandle.readSelector.columnEnd - fileHandle.readSelector.columnStart;
        inputInterface->initBuffer(numberOfColumns, info->timeLength);

        info->inputs.try_emplace(link.hexagonId, std::move(fileHandle));
    }

    // prepare outputs
    for (const TaskLink& link : outputs) {
        DataSetFileHandle fileHandle;
        ret = _fillTaskIo(fileHandle, clusterUuid, link, error);
        if (ret != OK) {
            return ret;
        }
        if (numberOfCycles > fileHandle.header.numberOfRows) {
            numberOfCycles = fileHandle.header.numberOfRows;
        }

        // resize number of output and size of io-buffer for the given data
        OutputInterface* outputInterface = &cluster->outputInterfaces[link.hexagonId];
        const uint64_t numberOfColumns
            = fileHandle.readSelector.columnEnd - fileHandle.readSelector.columnStart;
        outputInterface->initBuffer(numberOfColumns, 1);

        info->outputs.try_emplace(link.hexagonId, std::move(fileHandle));
    }

    // dataset with the lowest amount of rows defines the number of cycles
    for (auto& [hexagonName, file_handle] : info->inputs) {
        file_handle.readSelector.endRow = numberOfCycles;
    }
    for (auto& [hexagonName, file_handle] : info->outputs) {
        file_handle.readSelector.endRow = numberOfCycles;
    }

    // set number of cycles
    newTask->progress.totalNumberOfCycles = numberOfCycles;
    info->numberOfCycles = numberOfCycles - (info->timeLength - 1);

    cluster->stateMachine->goToNextState(PROCESS_TASK);

    return OK;
}

/**
 * @brief HanamiCore::createRequestTask
 * @param clusterUuid
 * @param taskName
 * @param inputs
 * @param outputs
 * @param timeLength
 * @param errorMessage
 * @return
 */
ReturnStatus
HanamiCore::createRequestTask(const std::string& clusterUuid,
                              const std::string& taskName,
                              const std::vector<TaskLink>& inputs,
                              const std::vector<TaskLink>& outputs,
                              const float timeLength,
                              std::string& errorMessage)
{
    Hanami::ErrorContainer error;
    ReturnStatus ret = ERROR;
    std::lock_guard<std::mutex> guard(m_clusterMutex);

    // get cluster
    Cluster* cluster = ClusterHandler::getInstance()->getCluster(clusterUuid);
    if (cluster == nullptr) {
        error.addMessage("Cluster with UUID '" + clusterUuid + "'not found");
        return INVALID_INPUT;
    }

    // create new request-task
    Task* newTask = cluster->addNewTask();
    if (newTask == nullptr) {
        return ERROR;
    }

    newTask->name = taskName;
    newTask->type = REQUEST_TASK;
    newTask->progress.queuedTimeStamp = std::chrono::system_clock::now();
    newTask->info = RequestInfo();
    RequestInfo* taskInfo = &std::get<RequestInfo>(newTask->info);
    taskInfo->timeLength = timeLength;
    u_int64_t numberOfCycles = std::numeric_limits<uint64_t>::max();

    // prepare inputs
    for (const TaskLink& link : inputs) {
        DataSetFileHandle fileHandle;
        ret = _fillTaskIo(fileHandle, clusterUuid, link, error);
        if (ret != OK) {
            return ret;
        }
        if (numberOfCycles > fileHandle.header.numberOfRows) {
            numberOfCycles = fileHandle.header.numberOfRows;
        }

        // resize number of inputs and size of io-buffer for the given data
        InputInterface* inputInterface = &cluster->inputInterfaces[link.hexagonId];
        const uint64_t numberOfColumns
            = fileHandle.readSelector.columnEnd - fileHandle.readSelector.columnStart;
        inputInterface->initBuffer(numberOfColumns, taskInfo->timeLength);

        taskInfo->inputs.try_emplace(link.hexagonId, std::move(fileHandle));
    }

    // prepare outputs
    for (const TaskLink& link : outputs) {
        const uint64_t numberOfOutputs = cluster->outputInterfaces[link.hexagonId].ioBuffer.size();

        json description;
        json descriptionEntry;
        descriptionEntry["column_start"] = 0;
        descriptionEntry["column_end"] = numberOfOutputs;
        description[link.hexagonId] = descriptionEntry;

        ret = initNewDataSetFile(link.datasetFilePath,
                                 link.datasetName,
                                 description,
                                 FLOAT_TYPE,
                                 numberOfOutputs,
                                 error);
        if (ret != OK) {
            return ret;
        }

        DataSetFileHandle fileHandle;
        ret = _fillTaskIo(fileHandle, clusterUuid, link, error);
        if (ret != OK) {
            return ret;
        }
        if (numberOfCycles > fileHandle.header.numberOfRows) {
            numberOfCycles = fileHandle.header.numberOfRows;
        }

        // resize number of output and size of io-buffer for the given data
        OutputInterface* outputInterface = &cluster->outputInterfaces[link.hexagonId];
        const uint64_t numberOfColumns
            = fileHandle.readSelector.columnEnd - fileHandle.readSelector.columnStart;
        outputInterface->initBuffer(numberOfColumns, 1);

        taskInfo->results.try_emplace(link.hexagonId, std::move(fileHandle));
    }

    for (auto& [hexagonName, file_handle] : taskInfo->inputs) {
        file_handle.readSelector.endRow = numberOfCycles;
    }

    // set number of cycles
    taskInfo->numberOfCycles = numberOfCycles - (timeLength - 1);
    taskInfo->timeLength = timeLength;
    newTask->progress.totalNumberOfCycles = numberOfCycles;

    cluster->stateMachine->goToNextState(PROCESS_TASK);

    return OK;
}

/**
 * @brief HanamiCore::fillTaskIo
 *
 * @param fileHandle
 * @param clusterUuid
 * @param taskLink
 * @param error
 *
 * @return
 */
ReturnStatus
HanamiCore::_fillTaskIo(DataSetFileHandle& fileHandle,
                        const std::string& clusterUuid,
                        const TaskLink& taskLink,
                        Hanami::ErrorContainer& error)
{
    ReturnStatus ret = openDataSetFile(fileHandle, taskLink.datasetFilePath, error);
    if (ret != OK) {
        error.addMessage("Dataset-file '" + taskLink.datasetFilePath + "' not found");
        return ERROR;
    }

    if (fileHandle.description.contains(taskLink.columnName) == false) {
        error.addMessage("Dataset doesn't contain column with name '" + taskLink.columnName + "'");
        return INVALID_INPUT;
    }

    fileHandle.readSelector.columnStart
        = fileHandle.description[taskLink.columnName]["column_start"];
    fileHandle.readSelector.columnEnd = fileHandle.description[taskLink.columnName]["column_end"];

    return ret;
}

/**
 * @brief createRootObj
 * @return
 */
std::unique_ptr<HanamiCore>
createRootObj()
{
    return std::make_unique<HanamiCore>();
}
void
HanamiCore::my_cpp_function()
{
    std::cout << "my_cpp_function" << std::endl;
}

int
HanamiCore::area() const
{
    return 5 * 5;
}

int
HanamiCore::perimeter() const
{
    return 4 * 5;
}
