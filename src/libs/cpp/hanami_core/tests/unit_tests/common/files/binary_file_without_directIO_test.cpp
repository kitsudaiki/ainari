/**
 * @file    binary_file_test.cpp
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

#include "binary_file_without_directIO_test.h"

#include <src/common/files/binary_file.h>

#include <filesystem>

namespace Hanami
{

struct TestStruct {
    uint8_t a = 0;
    uint8_t b = 0;
    uint64_t c = 0;
} __attribute__((packed));

BinaryFile_withoutDirectIO_Test::BinaryFile_withoutDirectIO_Test()
    : Hanami::CompareTestHelper("BinaryFile_withoutDirectIO_Test")
{
    initTest();
    closeFile_test();
    allocateStorage_test();
    writeCompleteFile_test();
    readCompleteFile_test();
    writeDataIntoFile_test();
    readDataFromFile_test();
    closeTest();
}

/**
 * initTest
 */
void
BinaryFile_withoutDirectIO_Test::initTest()
{
    m_filePath = "/tmp/binaryFile_test.bin";
    deleteFile();
}

/**
 * closeFile_test
 */
void
BinaryFile_withoutDirectIO_Test::closeFile_test()
{
    std::string error;

    // init buffer and file
    DataBuffer buffer;
    BinaryFile binaryFile(m_filePath);

    // test close
    TEST_EQUAL(binaryFile.closeFile(error), true);
    TEST_EQUAL(binaryFile.closeFile(error), true);

    deleteFile();
}

/**
 * @brief BinaryFile_Test::updateFileSize_test
 */
void
BinaryFile_withoutDirectIO_Test::updateFileSize_test()
{
    std::string error;

    // init buffer and file
    DataBuffer buffer(5);
    BinaryFile binaryFile(m_filePath);
    binaryFile.allocateStorage(4 * 4000, error);
    binaryFile.closeFile(error);

    BinaryFile binaryFileNew(m_filePath);
    TEST_EQUAL(binaryFileNew.isOpen(), true);
    TEST_EQUAL(binaryFileNew.updateFileSize(error), true);
    TEST_EQUAL(binaryFileNew.fileSize, 4 * 4096);

    TEST_EQUAL(binaryFileNew.fileSize, binaryFileNew.fileSize);
}

/**
 * allocateStorage_test
 */
void
BinaryFile_withoutDirectIO_Test::allocateStorage_test()
{
    std::string error;

    // init buffer and file
    DataBuffer buffer;
    BinaryFile binaryFile(m_filePath);

    // test allocation
    TEST_EQUAL(binaryFile.allocateStorage(4 * 4000, error), true);
    TEST_EQUAL(binaryFile.allocateStorage(4 * 4000, error), true);
    TEST_EQUAL(binaryFile.allocateStorage(0, error), true);

    // check meta-data
    TEST_EQUAL(binaryFile.fileSize, 8 * 4000);

    binaryFile.closeFile(error);

    // negative test
    TEST_EQUAL(binaryFile.allocateStorage(4 * 4000, error), false);

    deleteFile();
}

/**
 * writeCompleteFile_test
 */
void
BinaryFile_withoutDirectIO_Test::writeCompleteFile_test()
{
    std::string error;

    // init buffer and file
    DataBuffer buffer(5);
    BinaryFile binaryFile(m_filePath);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(buffer, &testStruct);
    testStruct.a = 10;
    testStruct.c = 1234;
    addObject_DataBuffer(buffer, &testStruct);

    TEST_EQUAL(binaryFile.writeCompleteFile(buffer, error), true);
    TEST_EQUAL(std::filesystem::file_size(m_filePath), 2 * sizeof(TestStruct));

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * readCompleteFile_test
 */
void
BinaryFile_withoutDirectIO_Test::readCompleteFile_test()
{
    std::string error;

    // init buffer and file
    DataBuffer sourceBuffer(5);
    DataBuffer targetBuffer(5);
    BinaryFile binaryFile(m_filePath);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(sourceBuffer, &testStruct);
    testStruct.a = 10;
    testStruct.c = 1234;
    addObject_DataBuffer(sourceBuffer, &testStruct);

    // test with buffer-size unequal a multiple of the block-size
    sourceBuffer.usedBufferSize = 2 * sourceBuffer.blockSize + 1;
    targetBuffer.usedBufferSize = 2 * targetBuffer.blockSize + 1;

    binaryFile.writeCompleteFile(sourceBuffer, error);
    TEST_EQUAL(binaryFile.readCompleteFile(targetBuffer, error), true);

    // check if source and target-buffer are
    int ret = memcmp(sourceBuffer.data, targetBuffer.data, 2 * sourceBuffer.blockSize + 1);
    TEST_EQUAL(ret, 0);

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * @brief writeDataIntoFile_test
 */
void
BinaryFile_withoutDirectIO_Test::writeDataIntoFile_test()
{
    std::string error;

    DataBuffer targetBuffer(5);

    // init buffer and file
    DataBuffer sourceBuffer(5);
    BinaryFile binaryFile(m_filePath);
    binaryFile.allocateStorage(4 * 4096, error);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(sourceBuffer, &testStruct);
    sourceBuffer.usedBufferSize = 2000;
    addObject_DataBuffer(sourceBuffer, &testStruct);

    // write-tests
    TEST_EQUAL(binaryFile.writeDataIntoFile(sourceBuffer.data, 0, 4000, error), true);

    // negative tests
    TEST_EQUAL(binaryFile.writeDataIntoFile(sourceBuffer.data, 42000, 1000, error), false);
    TEST_EQUAL(binaryFile.writeDataIntoFile(sourceBuffer.data, 2000, 42000, error), false);

    TEST_EQUAL(binaryFile.readCompleteFile(targetBuffer, error), true);

    // check if source and target-buffer are
    int ret = memcmp(sourceBuffer.data, targetBuffer.data, 2 * sourceBuffer.blockSize + 1);
    TEST_EQUAL(ret, 0);

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * @brief readDataFromFile_test
 */
void
BinaryFile_withoutDirectIO_Test::readDataFromFile_test()
{
    std::string error;

    DataBuffer targetBuffer(5);

    // init buffer and file
    DataBuffer sourceBuffer(5);
    BinaryFile binaryFile(m_filePath);
    binaryFile.allocateStorage(4 * 4096, error);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(sourceBuffer, &testStruct);
    sourceBuffer.usedBufferSize = 2000;
    addObject_DataBuffer(sourceBuffer, &testStruct);

    // write-tests
    binaryFile.writeDataIntoFile(sourceBuffer.data, 0, 4000, error);
    TEST_EQUAL(binaryFile.readDataFromFile(targetBuffer.data, 0, 4000, error), true);

    // negative tests
    TEST_EQUAL(binaryFile.readDataFromFile(targetBuffer.data, 42000, 1000, error), false);
    TEST_EQUAL(binaryFile.readDataFromFile(targetBuffer.data, 2000, 42000, error), false);

    // check if source and target-buffer are
    int ret = memcmp(sourceBuffer.data, targetBuffer.data, 2 * sourceBuffer.blockSize + 1);
    TEST_EQUAL(ret, 0);

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * closeTest
 */
void
BinaryFile_withoutDirectIO_Test::closeTest()
{
    deleteFile();
}

/**
 * common usage to delete test-file
 */
void
BinaryFile_withoutDirectIO_Test::deleteFile()
{
    std::filesystem::path rootPathObj(m_filePath);
    if (std::filesystem::exists(rootPathObj)) {
        std::filesystem::remove(rootPathObj);
    }
}

}  // namespace Hanami
