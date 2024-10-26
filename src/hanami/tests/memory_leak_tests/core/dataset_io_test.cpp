/**
 * @file        dataset_io_test.cpp
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

#include "dataset_io_test.h"

#include <core/cluster/task.h>
#include <core/io/data_set/dataset_file_io.h>
#include <hanami_common/functions/file_functions.h>

namespace Hanami
{

DataSetIO_Test::DataSetIO_Test() : Hanami::MemoryLeakTestHelpter("DataSetIO_Test")
{
    m_input = json::array({1, 2, 3, 4, 5, 6, 7, 8, 9});
    assert(m_input.size() > 0 && m_input.size() % 3 == 0);

    write_test();
    read_test();
}

/**
 * @brief write_test
 */
void
DataSetIO_Test::write_test()
{
    json description;
    description["test"] = "asdf";
    const std::string descriptionStr = description.dump();

    REINIT_TEST();

    Hanami::ErrorContainer* error = new Hanami::ErrorContainer();
    DataSetFileHandle* fileHandle = new DataSetFileHandle(1);

    Hanami::deleteFileOrDir(m_testFilePath, *error);
    initNewDataSetFile(*fileHandle,
                       m_testFilePath,
                       m_fileName,
                       description,
                       UINT8_TYPE,
                       m_numberOfColumns,
                       *error);

    appendToDataSet<uint8_t>(*fileHandle, m_input, *error);
    fileHandle->writeRemainingBufferToFile(*error);

    delete error;
    delete fileHandle;

    CHECK_MEMORY();
}

/**
 * @brief read_test
 */
void
DataSetIO_Test::read_test()
{
    json description;
    description["test"] = "asdf";
    const std::string descriptionStr = description.dump();
    std::vector<float> output(3, 0.0f);

    REINIT_TEST();

    DataSetFileHandle* fileHandleCopy = new DataSetFileHandle();

    Hanami::ErrorContainer* error = new Hanami::ErrorContainer();
    DataSetFileHandle* fileHandle = new DataSetFileHandle(1);

    openDataSetFile(*fileHandle, m_testFilePath, *error);

    DataSetSelector selector;
    selector.columnStart = 1;
    selector.columnEnd = 3;
    selector.endRow = 3;
    fileHandle->readSelector = selector;

    uint64_t row = 0;

    getDataFromDataSet(output, *fileHandle, row, *error);
    getDataFromDataSet(output, *fileHandle, row, *error);
    getDataFromDataSet(output, *fileHandle, row, *error);

    *fileHandleCopy = std::move(*fileHandle);

    delete fileHandleCopy;
    delete fileHandle;
    delete error;

    CHECK_MEMORY();
}

}  // namespace Hanami
