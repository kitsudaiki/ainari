/**
 * @file        main.cpp
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

#include <src/common/logger.h>

#include "common/buffer/data_buffer_test.h"
#include "common/buffer/item_buffer_test.h"
#include "common/files/binary_file_with_directIO_test.h"
#include "common/files/binary_file_without_directIO_test.h"
#include "common/functions/file_functions_test.h"
#include "common/functions/string_functions_test.h"
#include "common/functions/vector_functions_test.h"
#include "common/items/table_item_test.h"
#include "common/logger_test.h"
#include "common/threading/thread_handler_test.h"
#include "core/cluster_io_convert_test.h"
#include "core/cluster_test.h"
#include "core/hanami_core_test.h"
#include "core/processing_test.h"

int
main()
{
    Hanami::initConsoleLogger(false);

    Hanami::DataBuffer_Test();
    Hanami::ItemBuffer_Test();

    Hanami::Stringfunctions_Test();
    Hanami::Vectorfunctions_Test();
    Hanami::Filefunctions_Test();

    Hanami::Logger_Test();

    Hanami::ThreadHandler_Test();

    Hanami::BinaryFile_withDirectIO_Test();
    Hanami::BinaryFile_withoutDirectIO_Test();

    Hanami::TableItem_test();

    ClusterIOConvert_Test();
    Cluster_Init_Test();
    Processing_Test();
    Hanami_Core_Test();

    return 0;
}
