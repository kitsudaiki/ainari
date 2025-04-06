/**
 * @file    data_buffer_test.h
 *
 * @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright  Apache License Version 2.0
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

#ifndef DATA_BUFFER_TEST_H
#define DATA_BUFFER_TEST_H

#include <hanami_common/test_helper/compare_test_helper.h>

namespace Hanami
{

class DataBuffer_Test : public Hanami::CompareTestHelper
{
   public:
    DataBuffer_Test();

   private:
    void constructor_test();
    void structSize_test();
    void copy_assingment_constructor_test();
    void copy_assingment_operator_test();
    void addObject_DataBuffer_test();
    void reset_DataBuffer_test();
    void getPosition_DataBuffer_test();
    void getObject_DataBuffer_test();

    void addData_DataBuffer_test();
    void allocateBlocks_DataBuffer_test();
};

}  // namespace Hanami

#endif  // DATABUFFER_TEST_H
