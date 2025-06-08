/**
 * @file        buffer_io.h
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

#ifndef BUFFERIO_H
#define BUFFERIO_H

#include <io/checkpoint/io_interface.h>
#include <src/common/buffer/data_buffer.h>

class BufferIO : public IO_Interface
{
   public:
    BufferIO();
    ~BufferIO();

    ReturnStatus writeClusterIntoBuffer(Hanami::DataBuffer& target,
                                        const Cluster& cluster,
                                        std::string& error);
    ReturnStatus readClusterFromBuffer(Cluster& cluster,
                                       Hanami::DataBuffer& input,
                                       LogicalHost* host,
                                       std::string& error);

   private:
    bool initializeTarget(const uint64_t size, std::string& error);
    bool writeFromLocalBuffer(const LocalBuffer& localBuffer, std::string&);
    bool readToLocalBuffer(LocalBuffer& localBuffer, std::string&);

    Hanami::DataBuffer* m_buffer = nullptr;
};

#endif  // BUFFERIO_H
