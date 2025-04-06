/**
 * @file        buffer_io.cpp
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

#include "buffer_io.h"

/**
 * @brief constructor
 */
BufferIO::BufferIO() : IO_Interface() {}

/**
 * @brief destructor
 */
BufferIO::~BufferIO() {}

/**
 * @brief serialize cluster into a buffer
 *
 * @param target target-buffer
 * @param cluster cluster to serialize
 * @param error reference for error-output
 *
 * @return OK-status, if successful, else ERROR-status
 */
ReturnStatus
BufferIO::writeClusterIntoBuffer(Hanami::DataBuffer& target,
                                 const Cluster& cluster,
                                 Hanami::ErrorContainer& error)
{
    m_buffer = &target;

    return serialize(cluster, error);
}

/**
 * @brief read a cluster from a buffer
 *
 * @param cluster target-cluster
 * @param input input-buffer with the data for the target
 * @param host initial host to attach the hexagons. if nullptr, use the first cpu-host
 * @param reference to error-output
 *
 * @return return-status based on the result of the process
 */
ReturnStatus
BufferIO::readClusterFromBuffer(Cluster& cluster,
                                Hanami::DataBuffer& input,
                                LogicalHost* host,
                                Hanami::ErrorContainer& error)
{
    m_buffer = &input;

    return deserialize(cluster, m_buffer->usedBufferSize, host, error);
}

/**
 * @brief initialize the file on the disc
 *
 * @param size total size necessary for the complete serialized cluster
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
BufferIO::initializeTarget(const uint64_t size, Hanami::ErrorContainer& error)
{
    if (Hanami::reset_DataBuffer(*m_buffer, Hanami::calcBytesToBlocks(size)) == false) {
        error.addMessage("Failed to reset and initialize buffer");
        return false;
    }

    return true;
}

/**
 * @brief write data from the local buffer to the disc
 *
 * @param localBuffer local buffer with the data to write
 * @param error reference for error-output
 *
 * @return always true
 */
bool
BufferIO::writeFromLocalBuffer(const LocalBuffer& localBuffer, Hanami::ErrorContainer&)
{
    uint8_t* u8Data = static_cast<uint8_t*>(m_buffer->data);
    memcpy(&u8Data[localBuffer.startPos], localBuffer.cache, localBuffer.size);
    m_buffer->usedBufferSize = localBuffer.startPos + localBuffer.size;

    return true;
}

/**
 * @brief read a new block of data from the disc into the local buffer
 *
 * @param localBuffer local-buffer where the data should be written into
 * @param error reference for error-output
 *
 * @return always true
 */
bool
BufferIO::readToLocalBuffer(LocalBuffer& localBuffer, Hanami::ErrorContainer&)
{
    uint8_t* u8Data = static_cast<uint8_t*>(m_buffer->data);
    memcpy(localBuffer.cache, &u8Data[localBuffer.startPos], localBuffer.size);

    return true;
}
