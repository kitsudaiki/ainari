/**
 * @file       stack_buffer_reserve.h
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

#ifndef STACK_BUFFER_RESERVE_H
#define STACK_BUFFER_RESERVE_H

#include <hanami_common/buffer/data_buffer.h>

#include <atomic>
#include <vector>

#define STACK_BUFFER_BLOCK_SIZE 256 * 1024

namespace Hanami
{
class StackBufferReserve_Test;

class StackBufferReserve
{
   public:
    static StackBufferReserve* getInstance();

    bool addBuffer(DataBuffer* buffer);
    uint64_t getNumberOfBuffers();
    DataBuffer* getBuffer();

   private:
    StackBufferReserve(const uint32_t reserveSize = 100);
    ~StackBufferReserve();

    uint32_t m_reserveSize = 0;
    std::vector<DataBuffer*> m_reserve;
    std::atomic_flag m_lock = ATOMIC_FLAG_INIT;

    static StackBufferReserve* m_stackBufferReserve;
    friend StackBufferReserve_Test;
};

}  // namespace Hanami

#endif  // STACK_BUFFER_RESERVE_H
