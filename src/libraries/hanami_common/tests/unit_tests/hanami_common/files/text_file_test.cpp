/**
 * @file    text_file_test.cpp
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

#include "text_file_test.h"

#include <hanami_common/files/text_file.h>

namespace Hanami
{

TextFile_Test::TextFile_Test() : Hanami::CompareTestHelper("TextFile_Test")
{
    initTest();
    writeFile_test();
    readFile_test();
    appendText_test();
    replaceLine_test();
    replaceContent_test();
    closeTest();
}

/**
 * initTest
 */
void
TextFile_Test::initTest()
{
    m_filePath = "/tmp/textFile_test.txt";

    std::filesystem::path rootPathObj(m_filePath);
    if (std::filesystem::exists(rootPathObj)) {
        std::filesystem::remove(rootPathObj);
    }
}

/**
 * writeFile_test
 */
void
TextFile_Test::writeFile_test()
{
    bool ret;
    ErrorContainer error;
    std::string content
        = "this is a test\n"
          "and this is a second line";

    // write content to file in three different cases
    ret = writeFile(m_filePath, content, error, false);
    TEST_EQUAL(ret, true);
    ret = writeFile(m_filePath, content, error, false);
    TEST_EQUAL(ret, false);
    ret = writeFile(m_filePath, content, error, true);
    TEST_EQUAL(ret, true);

    // cleanup
    deleteFile();
}

/**
 * readFile_test
 */
void
TextFile_Test::readFile_test()
{
    bool ret;
    ErrorContainer error;
    std::string content
        = "this is a test\n"
          "and this is a second line";

    // write initial file
    writeFile(m_filePath, content, error, true);

    // run function and check result
    std::string fileContent = "";
    ret = readFile(fileContent, m_filePath, error);
    TEST_EQUAL(ret, true);
    TEST_EQUAL(fileContent, content);

    // negative test: file not exist
    fileContent = "";
    ret = readFile(fileContent, m_filePath + "_fake", error);
    TEST_EQUAL(ret, false);

    // cleanup
    deleteFile();
}

/**
 * appendText_test
 */
void
TextFile_Test::appendText_test()
{
    bool ret;
    ErrorContainer error;
    std::string content
        = "this is a test\n"
          "and this is a second line";

    // write initial file
    writeFile(m_filePath, content, error, false);

    // run function
    ret = appendText(m_filePath, "\nasdfasdfasdf", error);
    TEST_EQUAL(ret, true);

    // read updated file
    std::string fileContent = "";
    ret = readFile(fileContent, m_filePath, error);
    TEST_EQUAL(ret, true);

    // check result
    std::string compare
        = "this is a test\n"
          "and this is a second line\n"
          "asdfasdfasdf";
    TEST_EQUAL(fileContent, compare);

    // cleanup
    deleteFile();
}

/**
 * replaceLine_test
 */
void
TextFile_Test::replaceLine_test()
{
    bool ret;
    ErrorContainer error;
    std::string content
        = "this is a test\n"
          "and this is a second line\n"
          "asdfasdfasdf";

    // write initial file
    writeFile(m_filePath, content, error, false);

    // run function
    ret = replaceLine(m_filePath, 2, "poi", error);
    TEST_EQUAL(ret, true);

    // read updated file
    std::string fileContent = "";
    ret = readFile(fileContent, m_filePath, error);
    TEST_EQUAL(ret, true);

    // check result
    std::string compare
        = "this is a test\n"
          "and this is a second line\n"
          "poi";

    TEST_EQUAL(fileContent, compare);

    // cleanup
    deleteFile();
}

/**
 * replaceContent_test
 */
void
TextFile_Test::replaceContent_test()
{
    bool ret;
    ErrorContainer error;
    std::string content
        = "this is a test\n"
          "and this is a second line\n"
          "poi";

    // write initial file
    writeFile(m_filePath, content, error, false);

    // run function
    ret = replaceContent(m_filePath, "poi", "nani", error);
    TEST_EQUAL(ret, true);

    // read updated file
    std::string fileContent = "";
    ret = readFile(fileContent, m_filePath, error);
    TEST_EQUAL(ret, true);

    // check result
    std::string compare
        = "this is a test\n"
          "and this is a second line\n"
          "nani";
    TEST_EQUAL(fileContent, compare);

    // cleanup
    deleteFile();
}

/**
 * closeTest
 */
void
TextFile_Test::closeTest()
{
    deleteFile();
}

/**
 * common usage to delete test-file
 */
void
TextFile_Test::deleteFile()
{
    std::filesystem::path rootPathObj(m_filePath);
    if (std::filesystem::exists(rootPathObj)) {
        std::filesystem::remove(rootPathObj);
    }
}

}  // namespace Hanami
