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

#include "binary_file_with_directIO_test.h"

#include <hanami_common/files/binary_file_direct.h>

namespace Hanami
{

struct TestStruct {
    uint8_t a = 0;
    uint8_t b = 0;
    uint64_t c = 0;
} __attribute__((packed));

BinaryFile_withDirectIO_Test::BinaryFile_withDirectIO_Test()
    : Hanami::CompareTestHelper("BinaryFile_withDirectIO_Test")
{
    initTest();
    closeFile_test();
    allocateStorage_test();
    writeSegment_test();
    readSegment_test();
    writeCompleteFile_test();
    readCompleteFile_test();
    closeTest();
}

/**
 * initTest
 */
void
BinaryFile_withDirectIO_Test::initTest()
{
    m_filePath = "/tmp/binaryFile_test.bin";
    deleteFile();
}

/**
 * closeFile_test
 */
void
BinaryFile_withDirectIO_Test::closeFile_test()
{
    // init buffer and file
    DataBuffer buffer;
    BinaryFileDirect binaryFile(m_filePath);
    ErrorContainer error;

    // test close
    TEST_EQUAL(binaryFile.closeFile(error), true);
    TEST_EQUAL(binaryFile.closeFile(error), true);

    deleteFile();
}

/**
 * @brief BinaryFile_Test::updateFileSize_test
 */
void
BinaryFile_withDirectIO_Test::updateFileSize_test()
{
    ErrorContainer error;

    // init buffer and file
    DataBuffer buffer(5);
    BinaryFileDirect binaryFile(m_filePath);
    binaryFile.allocateStorage(4, 4096, error);
    binaryFile.closeFile(error);

    BinaryFileDirect binaryFileNew(m_filePath);
    TEST_EQUAL(binaryFileNew.isOpen(), true);
    TEST_EQUAL(binaryFileNew.updateFileSize(error), true);
    TEST_EQUAL(binaryFileNew.m_totalFileSize, 4 * 4096);

    TEST_EQUAL(binaryFileNew.m_totalFileSize, binaryFileNew.m_totalFileSize);
}

/**
 * allocateStorage_test
 */
void
BinaryFile_withDirectIO_Test::allocateStorage_test()
{
    ErrorContainer error;

    // init buffer and file
    DataBuffer buffer;
    BinaryFileDirect binaryFile(m_filePath);

    // test allocation
    TEST_EQUAL(binaryFile.allocateStorage(4, 4096, error), true);
    TEST_EQUAL(binaryFile.allocateStorage(4, 4096, error), true);
    TEST_EQUAL(binaryFile.allocateStorage(0, 4096, error), true);

    // check meta-data
    TEST_EQUAL(binaryFile.m_totalFileSize, 8 * 4096);

    binaryFile.closeFile(error);

    // negative test
    TEST_EQUAL(binaryFile.allocateStorage(4, 4096, error), false);

    deleteFile();
}

/**
 * writeSegment_test
 */
void
BinaryFile_withDirectIO_Test::writeSegment_test()
{
    ErrorContainer error;

    // init buffer and file
    DataBuffer buffer(5);
    BinaryFileDirect binaryFile(m_filePath);
    binaryFile.allocateStorage(4, 4096, error);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(buffer, &testStruct);
    buffer.usedBufferSize = 2 * buffer.blockSize;
    addObject_DataBuffer(buffer, &testStruct);

    // write-tests
    TEST_EQUAL(binaryFile.writeSegment(buffer, 1, 1, 0, error), true);
    TEST_EQUAL(binaryFile.writeSegment(buffer, 2, 1, 2, error), true);
    TEST_EQUAL(binaryFile.writeSegment(buffer, 2, 0, 3, error), true);

    // negative tests
    TEST_EQUAL(binaryFile.writeSegment(buffer, 42, 1, 3, error), false);
    TEST_EQUAL(binaryFile.writeSegment(buffer, 2, 42, 3, error), false);
    TEST_EQUAL(binaryFile.writeSegment(buffer, 2, 1, 42, error), false);

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * readSegment_test
 */
void
BinaryFile_withDirectIO_Test::readSegment_test()
{
    ErrorContainer error;

    // init buffer and file
    DataBuffer buffer(5);
    BinaryFileDirect binaryFile(m_filePath);
    binaryFile.allocateStorage(4, 4096, error);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(buffer, &testStruct);
    testStruct.a = 10;
    testStruct.c = 1234;
    buffer.usedBufferSize = 2 * buffer.blockSize;
    addObject_DataBuffer(buffer, &testStruct);

    // write the two blocks of the buffer
    TEST_EQUAL(binaryFile.writeSegment(buffer, 1, 1, 0, error), true);
    TEST_EQUAL(binaryFile.writeSegment(buffer, 2, 1, 2, error), true);

    // clear orinial buffer
    memset(buffer.data, 0, buffer.totalBufferSize);
    testStruct.a = 0;
    testStruct.c = 0;

    // read the two blocks back
    TEST_EQUAL(binaryFile.readSegment(buffer, 1, 1, 1, error), true);
    TEST_EQUAL(binaryFile.readSegment(buffer, 2, 1, 3, error), true);
    TEST_EQUAL(binaryFile.readSegment(buffer, 2, 0, 3, error), true);

    // negative tests
    TEST_EQUAL(binaryFile.readSegment(buffer, 42, 1, 3, error), false);
    TEST_EQUAL(binaryFile.readSegment(buffer, 2, 42, 3, error), false);
    TEST_EQUAL(binaryFile.readSegment(buffer, 2, 1, 42, error), false);

    // copy and check the first block
    mempcpy(&testStruct,
            static_cast<uint8_t*>(buffer.data) + (1 * buffer.blockSize),
            sizeof(TestStruct));
    TEST_EQUAL(testStruct.a, 42);
    TEST_EQUAL(testStruct.c, 1337);

    // copy and check the second block
    mempcpy(&testStruct,
            static_cast<uint8_t*>(buffer.data) + (3 * buffer.blockSize),
            sizeof(TestStruct));
    TEST_EQUAL(testStruct.a, 10);
    TEST_EQUAL(testStruct.c, 1234);

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * writeCompleteFile_test
 */
void
BinaryFile_withDirectIO_Test::writeCompleteFile_test()
{
    ErrorContainer error;

    // init buffer and file
    DataBuffer buffer(5);
    BinaryFileDirect binaryFile(m_filePath);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(buffer, &testStruct);
    testStruct.a = 10;
    testStruct.c = 1234;
    addObject_DataBuffer(buffer, &testStruct);
    buffer.usedBufferSize = 2 * buffer.blockSize;

    TEST_EQUAL(binaryFile.writeCompleteFile(buffer, error), true);
    TEST_EQUAL(std::filesystem::file_size(m_filePath), 2 * buffer.blockSize);

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * readCompleteFile_test
 */
void
BinaryFile_withDirectIO_Test::readCompleteFile_test()
{
    ErrorContainer error;

    // init buffer and file
    DataBuffer sourceBuffer(5);
    DataBuffer targetBuffer(5);
    BinaryFileDirect binaryFile(m_filePath);

    // prepare test-buffer
    TestStruct testStruct;
    testStruct.a = 42;
    testStruct.c = 1337;
    addObject_DataBuffer(sourceBuffer, &testStruct);
    testStruct.a = 10;
    testStruct.c = 1234;
    addObject_DataBuffer(sourceBuffer, &testStruct);
    sourceBuffer.usedBufferSize = 2 * sourceBuffer.blockSize;
    targetBuffer.usedBufferSize = 2 * targetBuffer.blockSize;

    binaryFile.writeCompleteFile(sourceBuffer, error);
    TEST_EQUAL(binaryFile.readCompleteFile(targetBuffer, error), true);

    // check if source and target-buffer are
    int ret = memcmp(sourceBuffer.data, targetBuffer.data, 2 * sourceBuffer.blockSize);
    TEST_EQUAL(ret, 0);

    // cleanup
    TEST_EQUAL(binaryFile.closeFile(error), true);
    deleteFile();
}

/**
 * closeTest
 */
void
BinaryFile_withDirectIO_Test::closeTest()
{
    deleteFile();
}

/**
 * common usage to delete test-file
 */
void
BinaryFile_withDirectIO_Test::deleteFile()
{
    std::filesystem::path rootPathObj(m_filePath);
    if (std::filesystem::exists(rootPathObj)) {
        std::filesystem::remove(rootPathObj);
    }
}

}  // namespace Hanami
