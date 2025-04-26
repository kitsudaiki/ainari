/**
 * @file        cluster_io_convert_test.cpp
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

#include "cluster_io_convert_test.h"

#include <cluster/cluster_io_convert.h>
#include <src/common/logger.h>

ClusterIOConvert_Test::ClusterIOConvert_Test() : Hanami::CompareTestHelper("ClusterIOConvert_Test")
{
    convertPlain_test();
    convertBool_test();
    convertFloat_test();
    convertInt_test();
}

/**
 * @brief convertPlain_test
 */
void
ClusterIOConvert_Test::convertPlain_test()
{
    OutputInterface testInterface;
    testInterface.type = PLAIN_OUTPUT;
    testInterface.initBuffer(4, 1);

    TEST_EQUAL(testInterface.ioBuffer.size(), 4);
    TEST_EQUAL(testInterface.outputNeurons.size(), 4);

    testInterface.outputNeurons[0].outputVal = 42.0f;
    testInterface.outputNeurons[1].outputVal = 43.0f;
    testInterface.outputNeurons[2].outputVal = 44.0f;
    testInterface.outputNeurons[3].outputVal = 45.0f;

    convertOutputToBuffer(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 4);
    TEST_EQUAL(testInterface.ioBuffer[0], 42.0f);
    TEST_EQUAL(testInterface.ioBuffer[1], 43.0f);
    TEST_EQUAL(testInterface.ioBuffer[2], 44.0f);
    TEST_EQUAL(testInterface.ioBuffer[3], 45.0f);

    convertBufferToExpected(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 4);
    TEST_EQUAL(testInterface.outputNeurons[0].exprectedVal, 42.0f);
    TEST_EQUAL(testInterface.outputNeurons[1].exprectedVal, 43.0f);
    TEST_EQUAL(testInterface.outputNeurons[2].exprectedVal, 44.0f);
    TEST_EQUAL(testInterface.outputNeurons[3].exprectedVal, 45.0f);
}

/**
 * @brief convertBool_test
 */
void
ClusterIOConvert_Test::convertBool_test()
{
    OutputInterface testInterface;
    testInterface.type = BOOL_OUTPUT;
    testInterface.initBuffer(4, 1);

    TEST_EQUAL(testInterface.ioBuffer.size(), 4);
    TEST_EQUAL(testInterface.outputNeurons.size(), 4);

    testInterface.outputNeurons[0].outputVal = 0.1f;
    testInterface.outputNeurons[1].outputVal = 0.6f;
    testInterface.outputNeurons[2].outputVal = 0.3f;
    testInterface.outputNeurons[3].outputVal = 0.8f;

    convertOutputToBuffer(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 4);
    TEST_EQUAL(testInterface.ioBuffer[0], 0.0f);
    TEST_EQUAL(testInterface.ioBuffer[1], 1.0f);
    TEST_EQUAL(testInterface.ioBuffer[2], 0.0f);
    TEST_EQUAL(testInterface.ioBuffer[3], 1.0f);

    testInterface.ioBuffer[0] = 0.1f;
    testInterface.ioBuffer[1] = 0.6f;
    testInterface.ioBuffer[2] = 0.3f;
    testInterface.ioBuffer[3] = 0.8f;

    convertBufferToExpected(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 4);
    TEST_EQUAL(testInterface.outputNeurons[0].exprectedVal, 0.0f);
    TEST_EQUAL(testInterface.outputNeurons[1].exprectedVal, 1.0f);
    TEST_EQUAL(testInterface.outputNeurons[2].exprectedVal, 0.0f);
    TEST_EQUAL(testInterface.outputNeurons[3].exprectedVal, 1.0f);
}

/**
 * @brief convertFloat_test
 */
void
ClusterIOConvert_Test::convertFloat_test()
{
    OutputInterface testInterface;
    testInterface.type = FLOAT_OUTPUT;
    testInterface.initBuffer(2, 1);

    TEST_EQUAL(testInterface.ioBuffer.size(), 2);
    TEST_EQUAL(testInterface.outputNeurons.size(), 64);

    testInterface.outputNeurons[15].outputVal = 0.6f;
    testInterface.outputNeurons[16].outputVal = 0.1f;
    testInterface.outputNeurons[42].outputVal = 0.3f;
    testInterface.outputNeurons[43].outputVal = 0.8f;

    convertOutputToBuffer(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 2);

    convertBufferToExpected(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 2);
    TEST_EQUAL(testInterface.outputNeurons[15].exprectedVal, 1.0f);
    TEST_EQUAL(testInterface.outputNeurons[16].exprectedVal, 0.0f);
    TEST_EQUAL(testInterface.outputNeurons[42].exprectedVal, 0.0f);
    TEST_EQUAL(testInterface.outputNeurons[43].exprectedVal, 1.0f);
}

/**
 * @brief convertInt_test
 */
void
ClusterIOConvert_Test::convertInt_test()
{
    OutputInterface testInterface;
    testInterface.type = INT_OUTPUT;
    testInterface.initBuffer(2, 1);

    TEST_EQUAL(testInterface.ioBuffer.size(), 2);
    TEST_EQUAL(testInterface.outputNeurons.size(), 128);

    testInterface.outputNeurons[62].outputVal = 0.6f;
    testInterface.outputNeurons[63].outputVal = 0.1f;
    testInterface.outputNeurons[126].outputVal = 0.3f;
    testInterface.outputNeurons[127].outputVal = 0.8f;

    convertOutputToBuffer(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 2);
    TEST_EQUAL(testInterface.ioBuffer[0], 2.0f);
    TEST_EQUAL(testInterface.ioBuffer[1], 1.0f);

    convertBufferToExpected(
        &testInterface, &testInterface.ioBuffer[0], testInterface.ioBuffer.size());

    TEST_EQUAL(testInterface.ioBuffer.size(), 2);
    TEST_EQUAL(testInterface.outputNeurons[62].exprectedVal, 1.0f);
    TEST_EQUAL(testInterface.outputNeurons[63].exprectedVal, 0.0f);
    TEST_EQUAL(testInterface.outputNeurons[126].exprectedVal, 0.0f);
    TEST_EQUAL(testInterface.outputNeurons[127].exprectedVal, 1.0f);
}
