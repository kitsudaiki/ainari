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

#include <iostream>

/**
 * @brief handle plain output-values
 *
 * @param outputInterface reference to output-interface
 *
 * @return number of values in io-buffer
 */
uint64_t
_handlePlainOutput(OutputInterface* outputInterface, float* output, const uint64_t numberOfOutputs)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    uint64_t i = 0;
    while (i < upperBorder && i < numberOfOutputs) {
        output[i] = outputInterface->outputNeurons[i].outputVal;
        ++i;
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
_handleBoolOutput(OutputInterface* outputInterface, float* output, const uint64_t numberOfOutputs)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    uint64_t i = 0;
    while (i < upperBorder && i < numberOfOutputs) {
        const float val = outputInterface->outputNeurons[i].outputVal >= 0.5f;
        output[i] = val;
        ++i;
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
_handleIntOutput(OutputInterface* outputInterface, float* output, const uint64_t numberOfOutputs)
{
    OutputNeuron* neuron = nullptr;
    uint64_t val = 0;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 64;
    uint64_t i = 0;
    while (i < upperBorder && i < numberOfOutputs) {
        val = 0;

        for (uint64_t offset = 0; offset < 64; ++offset) {
            neuron = &outputInterface->outputNeurons[(i * 64) + offset];
            val = (val << 1) | static_cast<uint64_t>(neuron->outputVal >= 0.5f);
        }

        output[i] = val;
        ++i;
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
_handleFloatOutput(OutputInterface* outputInterface, float* output, const uint64_t numberOfOutputs)
{
    OutputNeuron* neuron = nullptr;
    uint32_t val = 0;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 32;
    uint64_t i = 0;
    while (i < upperBorder && i < numberOfOutputs) {
        val = 0;

        for (uint64_t offset = 0; offset < 32; ++offset) {
            neuron = &outputInterface->outputNeurons[(i * 32) + offset];
            val = (val << 1) | static_cast<uint32_t>(neuron->outputVal >= 0.5f);
        }

        float* floatVal = static_cast<float*>(static_cast<void*>(&val));
        output[i] = *floatVal;
        ++i;
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
convertOutputToBuffer(OutputInterface* outputInterface,
                      float* output,
                      const uint64_t numberOfOutputs)
{
    switch (outputInterface->type) {
        case PLAIN_OUTPUT:
            return _handlePlainOutput(outputInterface, output, numberOfOutputs);
        case BOOL_OUTPUT:
            return _handleBoolOutput(outputInterface, output, numberOfOutputs);
        case INT_OUTPUT:
            return _handleIntOutput(outputInterface, output, numberOfOutputs);
        case FLOAT_OUTPUT:
            return _handleFloatOutput(outputInterface, output, numberOfOutputs);
        default:
            return _handlePlainOutput(outputInterface, output, numberOfOutputs);
    }
}

/**
 * @brief prepare expected-value for plain output
 *
 * @param outputInterface reference to output-interface
 */
void
_handlePlainExpected(OutputInterface* outputInterface,
                     const float* output,
                     const uint64_t numberOfOutputs)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    for (uint64_t i = 0; i < upperBorder; ++i) {
        outputInterface->outputNeurons[i].exprectedVal = output[i];
    }
}

/**
 * @brief prepare expected-value for bool output
 *
 * @param outputInterface reference to output-interface
 */
void
_handleBoolExpected(OutputInterface* outputInterface,
                    const float* output,
                    const uint64_t numberOfOutputs)
{
    const uint64_t upperBorder = outputInterface->outputNeurons.size();
    for (uint64_t i = 0; i < upperBorder; ++i) {
        outputInterface->outputNeurons[i].exprectedVal = output[i] >= 0.5f;
    }
}

/**
 * @brief prepare expected-value for uint64 output
 *
 * @param outputInterface reference to output-interface
 */
void
_handleIntExpected(OutputInterface* outputInterface,
                   const float* output,
                   const uint64_t numberOfOutputs)
{
    OutputNeuron* neuron = nullptr;
    uint64_t val = 0;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 64;
    for (uint64_t i = 0; i < upperBorder; ++i) {
        val = output[i];
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
_handleFloatExpected(OutputInterface* outputInterface,
                     const float* output,
                     const uint64_t numberOfOutputs)
{
    OutputNeuron* neuron = nullptr;
    const uint32_t* val;

    const uint64_t upperBorder = outputInterface->outputNeurons.size() / 32;
    for (uint64_t i = 0; i < upperBorder; ++i) {
        val = static_cast<const uint32_t*>(static_cast<const void*>(&output[i]));
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
convertBufferToExpected(OutputInterface* outputInterface,
                        const float* output,
                        const uint64_t numberOfOutputs)
{
    switch (outputInterface->type) {
        case PLAIN_OUTPUT:
            _handlePlainExpected(outputInterface, output, numberOfOutputs);
            break;
        case BOOL_OUTPUT:
            _handleBoolExpected(outputInterface, output, numberOfOutputs);
            break;
        case INT_OUTPUT:
            _handleIntExpected(outputInterface, output, numberOfOutputs);
            break;
        case FLOAT_OUTPUT:
            _handleFloatExpected(outputInterface, output, numberOfOutputs);
            break;
        default:
            _handlePlainExpected(outputInterface, output, numberOfOutputs);
            break;
    }
}
