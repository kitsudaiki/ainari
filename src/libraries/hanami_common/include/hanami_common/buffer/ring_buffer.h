/**
 * @file       ring_buffer.h
 *
 *  @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *  @copyright  Apache License Version 2.0
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
#ifndef RING_BUFFER_H
#define RING_BUFFER_H

#include <hanami_common/buffer/data_buffer.h>
#include <string.h>

namespace Hanami
{

// if this buffer is too big, it doesn't fit into the cpu-cache anymore and this makes the buffer
// much slower
#define DEFAULT_RING_BUFFER_SIZE 2 * 1024 * 1024

struct RingBuffer {
    uint8_t* data = nullptr;
    uint64_t totalBufferSize = DEFAULT_RING_BUFFER_SIZE;
    uint64_t readPosition = 0;
    uint64_t usedSize = 0;

    // backup-buffer to collect messages, which are splitted
    // in the data-object
    uint8_t* overflowBuffer = nullptr;

    RingBuffer(const uint64_t ringBufferSize = DEFAULT_RING_BUFFER_SIZE)
    {
        totalBufferSize = ringBufferSize;
        data = static_cast<uint8_t*>(alignedMalloc(4096, ringBufferSize));
        overflowBuffer = static_cast<uint8_t*>(alignedMalloc(4096, ringBufferSize));
    }

    ~RingBuffer()
    {
        alignedFree(data, totalBufferSize);
        alignedFree(overflowBuffer, totalBufferSize);
    }
};

/**
 * @brief get position to append new data
 *
 * @param ringBuffer reference to ringbuffer-object
 *
 * @return position within the ring-buffer in bytes
 */
inline uint64_t
getWritePosition_RingBuffer(RingBuffer& ringBuffer)
{
    return (ringBuffer.readPosition + ringBuffer.usedSize) % ringBuffer.totalBufferSize;
}

/**
 * @brief Get number of bytes until the the end of the byte-array or until the the read-positoin.
 *        It doesn't return the total available space of the ring-buffer.
 *
 * @param ringBuffer reference to ringbuffer-object
 *
 * @return number of bytes until next blocker (end of array or read-position)
 */
inline uint64_t
getSpaceToEnd_RingBuffer(RingBuffer& ringBuffer)
{
    const uint64_t writePosition = getWritePosition_RingBuffer(ringBuffer);

    uint64_t spaceToEnd = ringBuffer.totalBufferSize - writePosition;
    if (writePosition < ringBuffer.readPosition) {
        spaceToEnd = ringBuffer.readPosition - writePosition;
    }

    return spaceToEnd;
}

/**
 * @brief write data into the ring-buffer
 *
 * @param ringBuffer reference to ringbuffer-object
 * @param data pointer to the new data
 * @param dataSize size of the new data
 *
 * @return false, if data are bigger than the available space inside the buffer, else true
 */
inline bool
addData_RingBuffer(RingBuffer& ringBuffer, const void* data, const uint64_t dataSize)
{
    if (dataSize + ringBuffer.usedSize > ringBuffer.totalBufferSize) {
        return false;
    }

    const uint64_t writePosition = getWritePosition_RingBuffer(ringBuffer);
    const uint64_t spaceToEnd = getSpaceToEnd_RingBuffer(ringBuffer);

    if (dataSize <= spaceToEnd) {
        memcpy(&ringBuffer.data[writePosition], data, dataSize);
    }
    else {
        const uint64_t remaining = dataSize - spaceToEnd;
        const uint8_t* dataPos = static_cast<const uint8_t*>(data);

        memcpy(&ringBuffer.data[writePosition], &dataPos[0], spaceToEnd);
        memcpy(&ringBuffer.data[0], &dataPos[spaceToEnd], remaining);
    }

    ringBuffer.usedSize += dataSize;

    return true;
}

/**
 * @brief add an object to the buffer
 *
 * @param ringBuffer reference to ringbuffer-object
 * @param data pointer to the object, which shoulb be written to the buffer
 *
 * @return false if precheck or allocation failed, else true
 */
template <typename T>
inline bool
addObject_RingBuffer(RingBuffer& ringBuffer, T* data)
{
    return addData_RingBuffer(ringBuffer, data, sizeof(T));
}

/**
 * get a pointer to the complete monolitic block of the ring-buffer
 *
 * @param ringBuffer reference to ringbuffer-object
 * @param size size of the requested block
 *
 * @return pointer to the beginning of the requested datablock, or nullptr if the requested
 *         block is too big
 */
inline const uint8_t*
getDataPointer_RingBuffer(const RingBuffer& ringBuffer, const uint64_t size)
{
    if (ringBuffer.usedSize < size) {
        return nullptr;
    }

    const uint64_t startPosition = ringBuffer.readPosition % ringBuffer.totalBufferSize;

    // check if requested datablock is splitet
    if (startPosition + size > ringBuffer.totalBufferSize) {
        // copy the two parts of the requested block into the overflow-buffer
        const uint64_t firstPart = size - ((startPosition + size) % ringBuffer.totalBufferSize);
        memcpy(&ringBuffer.overflowBuffer[0], &ringBuffer.data[startPosition], firstPart);
        memcpy(&ringBuffer.overflowBuffer[firstPart], &ringBuffer.data[0], size - firstPart);
        return &ringBuffer.overflowBuffer[0];
    }

    return &ringBuffer.data[startPosition];
}

/**
 * @brief move the read-position of the ring-buffer forward
 *
 * @param ringBuffer reference to ringbuffer-object
 * @param numberOfBytes number of bytes to move forward
 */
inline void
moveForward_RingBuffer(RingBuffer& ringBuffer, const uint64_t numberOfBytes)
{
    ringBuffer.readPosition
        = (ringBuffer.readPosition + numberOfBytes) % ringBuffer.totalBufferSize;
    ringBuffer.usedSize -= numberOfBytes;
}

/**
 * @brief get a pointer to an object at the beginning of the ring-buffer
 *
 * @param ringBuffer reference to ringbuffer-object
 *
 * @return pointer to requested object within the ring-buffer, but nullptr if there are not enough
 *         data within the ring-buffer for the requested object
 */
template <typename T>
inline const T*
getObject_RingBuffer(const RingBuffer& ringBuffer)
{
    const void* data = static_cast<const void*>(getDataPointer_RingBuffer(ringBuffer, sizeof(T)));

    return static_cast<const T*>(data);
}

}  // namespace Hanami

#endif  // RING_BUFFER_H
