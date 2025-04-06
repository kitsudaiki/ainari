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

#include <hanami_common/functions/file_functions.h>
#include <io/data_set/dataset_file_io.h>

DataSetIO_Test::DataSetIO_Test() : Hanami::CompareTestHelper("DataSetIO_Test")
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
    Hanami::ErrorContainer error;
    DataSetFileHandle fileHandle(1);

    Hanami::deleteFileOrDir(m_testFilePath, error);

    json description;
    description["test"] = "asdf";
    const std::string descriptionStr = description.dump();

    TEST_EQUAL(initNewDataSetFile(fileHandle,
                                  m_testFilePath,
                                  m_fileName,
                                  description,
                                  UINT8_TYPE,
                                  m_numberOfColumns,
                                  error),
               OK);
    TEST_EQUAL(initNewDataSetFile(fileHandle,
                                  m_testFilePath,
                                  m_fileName,
                                  description,
                                  UINT8_TYPE,
                                  m_numberOfColumns,
                                  error),
               INVALID_INPUT);

    TEST_EQUAL(fileHandle.header.name.getName(), m_fileName);
    TEST_EQUAL(fileHandle.header.fileSize, sizeof(DataSetHeader) + descriptionStr.size());
    TEST_EQUAL(fileHandle.header.dataType, UINT8_TYPE);
    TEST_EQUAL(fileHandle.header.numberOfColumns, m_numberOfColumns);
    TEST_EQUAL(fileHandle.header.numberOfRows, 0);

    TEST_EQUAL(appendToDataSet<uint8_t>(fileHandle, m_input, error), OK);
    TEST_EQUAL(fileHandle.writeRemainingBufferToFile(error), true);

    TEST_EQUAL(fileHandle.header.name.getName(), m_fileName);
    TEST_EQUAL(fileHandle.header.fileSize,
               sizeof(DataSetHeader) + descriptionStr.size() + (m_input.size() * sizeof(uint8_t)));
    TEST_EQUAL(fileHandle.header.dataType, UINT8_TYPE);
    TEST_EQUAL(fileHandle.header.numberOfColumns, m_numberOfColumns);
    TEST_EQUAL(fileHandle.header.numberOfRows, 3);
    TEST_EQUAL(fileHandle.description, description);
}

/**
 * @brief read_test
 */
void
DataSetIO_Test::read_test()
{
    Hanami::ErrorContainer error;
    DataSetFileHandle fileHandle(1);

    json description;
    description["test"] = "asdf";
    const std::string descriptionStr = description.dump();

    TEST_EQUAL(openDataSetFile(fileHandle, m_testFilePath, error), OK);

    TEST_EQUAL(fileHandle.header.name.getName(), m_fileName);
    TEST_EQUAL(fileHandle.header.fileSize,
               sizeof(DataSetHeader) + descriptionStr.size() + (m_input.size() * sizeof(uint8_t)));
    TEST_EQUAL(fileHandle.header.dataType, UINT8_TYPE);
    TEST_EQUAL(fileHandle.header.numberOfColumns, m_numberOfColumns);
    TEST_EQUAL(fileHandle.header.numberOfRows, 3);

    std::vector<float> output(3, 0.0f);
    DataSetSelector selector;
    selector.columnStart = 1;
    selector.columnEnd = 3;
    selector.endRow = 3;
    fileHandle.readSelector = selector;

    uint64_t row = 0;

    TEST_EQUAL(getDataFromDataSet(output, fileHandle, row, error), OK);
    TEST_EQUAL(output[0], 2.0f);
    TEST_EQUAL(output[1], 3.0f);

    row = 1;
    TEST_EQUAL(getDataFromDataSet(output, fileHandle, row, error), OK);
    TEST_EQUAL(output[0], 5.0f);
    TEST_EQUAL(output[1], 6.0f);

    row = 2;
    TEST_EQUAL(getDataFromDataSet(output, fileHandle, row, error), OK);
    TEST_EQUAL(output[0], 8.0f);
    TEST_EQUAL(output[1], 9.0f);

    TEST_EQUAL(getDataFromDataSet(output, fileHandle, 4, error), INVALID_INPUT);
    output.resize(1);
    TEST_EQUAL(getDataFromDataSet(output, fileHandle, 0, error), INVALID_INPUT);
}
