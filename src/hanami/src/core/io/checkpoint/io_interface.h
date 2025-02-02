/**
 * @file        io_interface.h
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

#ifndef IO_INTERFACE_H
#define IO_INTERFACE_H

#include <assert.h>
#include <core/cluster/objects.h>
#include <hanami_common/logger.h>
#include <stdint.h>

class Cluster;
namespace Hanami
{
struct DataBuffer;
}

#define LOCAL_BUFFER_SIZE 1024 * 4096

class IO_Interface
{
   protected:
    struct LocalBuffer {
        uint8_t cache[LOCAL_BUFFER_SIZE];
        uint64_t startPos = 0;
        uint64_t size = 0;
        uint64_t totalSize = 0;
    };

    struct InputEntry {
        Hanami::NameEntry name;
        uint32_t numberOfInputs = 0;
        uint32_t targetHexagonId = UNINIT_STATE_32;
    };

    struct OutputEntry {
        Hanami::NameEntry name;
        OutputType type = PLAIN_OUTPUT;
        uint32_t numberOfOutputs = 0;
        uint32_t numberOfWeightBlocks = 0;
        uint32_t targetHexagonId = UNINIT_STATE_32;
    };

    struct HexagonEntry {
        HexagonHeader header;
        uint64_t hexagonSize = 0;

        uint64_t axonBlocksPos = 0;
        uint64_t numberOfAxonBytes = 0;

        uint64_t blocksPos = 0;
        uint64_t numberOfSynapseBytes = 0;

        uint64_t inputInterfacesPos = 0;
        uint64_t numberOfInputsBytes = 0;

        uint64_t outputsInterfacesPos = 0;
        uint64_t numberOfOutputBytes = 0;
    };

    IO_Interface();
    virtual ~IO_Interface();

    ReturnStatus serialize(const Cluster& cluster, Hanami::ErrorContainer& error);
    ReturnStatus deserialize(Cluster& cluster,
                             const uint64_t totalSize,
                             LogicalHost* host,
                             Hanami::ErrorContainer& error);

   private:
    /**
     * @brief add a new object to the buffer
     *
     * @param data pointer to the object, which should be written into the target
     * @param error reference for error-output
     *
     * @return  true, if successful, else false
     */
    template <typename T>
    inline bool addObjectToLocalBuffer(T* data, Hanami::ErrorContainer& error)
    {
        assert(sizeof(T) < LOCAL_BUFFER_SIZE);
        if (sizeof(T) + m_localBuffer.size > LOCAL_BUFFER_SIZE) {
            if (writeFromLocalBuffer(m_localBuffer, error) == false) {
                error.addMessage("Failed to write local buffer to target");
                return false;
            }

            m_localBuffer.startPos += m_localBuffer.size;
            m_localBuffer.size = 0;
        }

        memcpy(&m_localBuffer.cache[m_localBuffer.size], data, sizeof(T));
        m_localBuffer.size += sizeof(T);

        return true;
    }

    /**
     * @brief get an object from the local-buffer of the underlying target
     *
     * @param bytePosition total byte-position within the target
     * @param data pointer to the object, which should be written to the target
     * @param error reference for error-output
     *
     * @return ReturnStatus based on the outcome
     */
    template <typename T>
    inline ReturnStatus getObjectFromLocalBuffer(uint64_t& bytePosition,
                                                 T* data,
                                                 Hanami::ErrorContainer& error)
    {
        if (bytePosition + sizeof(T) > m_localBuffer.size + m_localBuffer.startPos) {
            // update position in buffer
            m_localBuffer.startPos = bytePosition;

            // update size in buffer
            const uint64_t remain = m_localBuffer.totalSize - m_localBuffer.startPos;
            u_int64_t size = LOCAL_BUFFER_SIZE;
            if (remain < LOCAL_BUFFER_SIZE) {
                size = remain;
            }
            m_localBuffer.size = size;

            // handle special-case, which should never appear, because when this function is called,
            // it is expected, that there are more data in the target
            if (size <= 0) {
                error.addMessage("Input-data invalid");
                return INVALID_INPUT;
            }
            if (m_localBuffer.totalSize < m_localBuffer.startPos + m_localBuffer.size) {
                error.addMessage("Input-data invalid");
                return INVALID_INPUT;
            }

            if (readToLocalBuffer(m_localBuffer, error) == false) {
                error.addMessage("Failed to read cluster-data from target into local buffer");
                return ERROR;
            }
        }

        // copy data from buffer to target-object
        const uint64_t localPos = bytePosition - m_localBuffer.startPos;
        assert(localPos < LOCAL_BUFFER_SIZE);  // if localPos is too big, then there is something
                                               // totally wrong in the code
        memcpy(data, &m_localBuffer.cache[localPos], sizeof(T));
        bytePosition += sizeof(T);

        return OK;
    }

    virtual bool initializeTarget(const uint64_t size, Hanami::ErrorContainer& error) = 0;
    virtual bool writeFromLocalBuffer(const LocalBuffer& localBuffer, Hanami::ErrorContainer& error)
        = 0;
    virtual bool readToLocalBuffer(LocalBuffer& localBuffer, Hanami::ErrorContainer& error) = 0;

    uint64_t getClusterSize(const Cluster& cluster) const;
    uint64_t getHexagonSize(const Hexagon& hexagon) const;

    ReturnStatus serializeHexagon(const Hexagon& hexagon, Hanami::ErrorContainer& error);
    ReturnStatus deserializeHexagon(Hexagon& hexagon,
                                    uint64_t& positionPtr,
                                    Hanami::ErrorContainer& error);

    bool checkHexagonEntry(const HexagonEntry& hexagonEntry);
    HexagonEntry createHexagonEntry(const Hexagon& hexagon);

    void clearBlocks(Hexagon& hexagon);
    void initLocalBuffer(const uint64_t totalSize);

    LocalBuffer m_localBuffer;
};

#endif  // IO_INTERFACE_H
