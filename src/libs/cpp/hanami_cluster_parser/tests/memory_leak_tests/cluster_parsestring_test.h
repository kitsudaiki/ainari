/**
 * @file       cluster_parsestring_test.h
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

#ifndef CLUSTER_PARSESTRING_TEST_H
#define CLUSTER_PARSESTRING_TEST_H

#include <hanami_common/test_helper/memory_leak_test_helper.h>

namespace Hanami
{

class Cluster_ParseString_Test : public Hanami::MemoryLeakTestHelpter
{
   public:
    Cluster_ParseString_Test();

   private:
    void parseString_test();
};

}  // namespace Hanami

#endif  // CLUSTER_PARSESTRING_TEST_H
