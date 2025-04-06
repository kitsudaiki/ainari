/**
 * @file        cluster_io_convert.cpp
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

#include "cluster_io_convert.h"

/**
 * @brief handle plain output-values
 *
 * @param outputInterface reference to output-interface
 *
 * @return number of values in io-buffer
 */
uint64_t
_handlePlainOutput(OutputInterface* outputInterface)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    for (uint64_t i = 0; i < upperBorder; ++i) {
        outputInterface->ioBuffer[i] = outputInterface->outputNeurons[i].outputVal;
    }

    return upperBorder;
}

/**
 * @brief handle bool output-values
 *
 * @param outputInterface reference to output-interface
 *
 * @return number of values in io-buffer
 */
uint64_t
_handleBoolOutput(OutputInterface* outputInterface)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    for (uint64_t i = 0; i < upperBorder; ++i) {
        const float val = outputInterface->outputNeurons[i].outputVal >= 0.5f;
        outputInterface->ioBuffer[i] = val;
    }

    return upperBorder;
}

/**
 * @brief handle uint64 output-values by combining all bits of the outputs into
 *        uint64-values, which are pushed into the io-buffer
 *
 * @param outputInterface reference to output-interface
 *
 * @return number of values in io-buffer
 */
uint64_t
_handleIntOutput(OutputInterface* outputInterface)
{
    OutputNeuron* neuron = nullptr;
    uint64_t val = 0;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 64;
    for (uint64_t i = 0; i < upperBorder; ++i) {
        val = 0;

        for (uint64_t offset = 0; offset < 64; ++offset) {
            neuron = &outputInterface->outputNeurons[(i * 64) + offset];
            val = (val << 1) | static_cast<uint64_t>(neuron->outputVal >= 0.5f);
        }

        outputInterface->ioBuffer[i] = val;
    }

    return upperBorder;
}

/**
 * @brief handle float output-values by combining all bits of the outputs into
 *        float-values, which are pushed into the io-buffer
 *
 * @param outputInterface reference to output-interface
 *
 * @return number of values in io-buffer
 */
uint64_t
_handleFloatOutput(OutputInterface* outputInterface)
{
    OutputNeuron* neuron = nullptr;
    uint32_t val = 0;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 32;
    for (uint64_t i = 0; i < upperBorder; ++i) {
        val = 0;

        for (uint64_t offset = 0; offset < 32; ++offset) {
            neuron = &outputInterface->outputNeurons[(i * 32) + offset];
            val = (val << 1) | static_cast<uint32_t>(neuron->outputVal >= 0.5f);
        }

        float* floatVal = static_cast<float*>(static_cast<void*>(&val));
        outputInterface->ioBuffer[i] = *floatVal;
    }

    return upperBorder;
}

/**
 * @brief convert output based on the type and move the result into the io-buffer
 *
 * @param outputInterface reference to output-interface
 *
 * @return number of values in io-buffer
 */
uint64_t
convertOutputToBuffer(OutputInterface* outputInterface)
{
    switch (outputInterface->type) {
        case PLAIN_OUTPUT:
            return _handlePlainOutput(outputInterface);
        case BOOL_OUTPUT:
            return _handleBoolOutput(outputInterface);
        case INT_OUTPUT:
            return _handleIntOutput(outputInterface);
        case FLOAT_OUTPUT:
            return _handleFloatOutput(outputInterface);
        default:
            return _handlePlainOutput(outputInterface);
    }
}

/**
 * @brief prepare expected-value for plain output
 *
 * @param outputInterface reference to output-interface
 */
void
_handlePlainExpected(OutputInterface* outputInterface)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    for (uint64_t i = 0; i < upperBorder; ++i) {
        outputInterface->outputNeurons[i].exprectedVal = outputInterface->ioBuffer[i];
    }
}

/**
 * @brief prepare expected-value for bool output
 *
 * @param outputInterface reference to output-interface
 */
void
_handleBoolExpected(OutputInterface* outputInterface)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    for (uint64_t i = 0; i < upperBorder; ++i) {
        outputInterface->outputNeurons[i].exprectedVal = outputInterface->ioBuffer[i] >= 0.5f;
    }
}

/**
 * @brief prepare expected-value for uint64 output
 *
 * @param outputInterface reference to output-interface
 */
void
_handleIntExpected(OutputInterface* outputInterface)
{
    OutputNeuron* neuron = nullptr;
    uint64_t val = 0;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 64;
    for (uint64_t i = 0; i < upperBorder; ++i) {
        val = outputInterface->ioBuffer[i];
        for (uint64_t offset = 0; offset < 64; ++offset) {
            neuron = &outputInterface->outputNeurons[(i * 64) + (63 - offset)];
            neuron->exprectedVal = (val >> offset) & 1;
        }
    }
}

/**
 * @brief prepare expected-value for float output
 *
 * @param outputInterface reference to output-interface
 */
void
_handleFloatExpected(OutputInterface* outputInterface)
{
    OutputNeuron* neuron = nullptr;
    uint32_t* val;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 32;
    for (uint64_t i = 0; i < upperBorder; ++i) {
        val = static_cast<uint32_t*>(static_cast<void*>(&outputInterface->ioBuffer[i]));
        for (uint64_t offset = 0; offset < 32; ++offset) {
            neuron = &outputInterface->outputNeurons[(i * 32) + (31 - offset)];
            neuron->exprectedVal = (*val >> offset) & 1;
        }
    }
}

/**
 * @brief convert value of the io-buffer based on the type and move the result
 *        into the expected-field of the output
 *
 * @param outputInterface reference to output-interface
 */
void
convertBufferToExpected(OutputInterface* outputInterface)
{
    switch (outputInterface->type) {
        case PLAIN_OUTPUT:
            _handlePlainExpected(outputInterface);
            break;
        case BOOL_OUTPUT:
            _handleBoolExpected(outputInterface);
            break;
        case INT_OUTPUT:
            _handleIntExpected(outputInterface);
            break;
        case FLOAT_OUTPUT:
            _handleFloatExpected(outputInterface);
            break;
        default:
            _handlePlainExpected(outputInterface);
            break;
    }
}
