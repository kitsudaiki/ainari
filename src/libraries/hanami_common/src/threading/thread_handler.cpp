/**
 * @file       thread_handler.cpp
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

#include <hanami_common/threading/event.h>
#include <hanami_common/threading/thread.h>
#include <hanami_common/threading/thread_handler.h>

namespace Hanami
{

Hanami::ThreadHandler* ThreadHandler::m_instance = new ThreadHandler();

ThreadHandler::ThreadHandler() {}

/**
 * @brief static methode to get instance of the interface
 *
 * @return pointer to the static instance
 */
ThreadHandler*
ThreadHandler::getInstance()
{
    return m_instance;
}

/**
 * @brief get names of all registered threads
 *
 * @return list of names of all registred threads
 */
const std::vector<std::string>
ThreadHandler::getRegisteredNames()
{
    std::unique_lock<std::mutex> lock(m_mutex);

    std::vector<std::string> result;
    for (const auto& [id, thread] : m_allThreads) {
        result.push_back(id);
    }

    return result;
}

/**
 * @brief get all registered thread by a specific name
 *
 * @param threadName name of the requested thread
 *
 * @return list with all threads, which were registered under the name
 */
const std::vector<Thread*>
ThreadHandler::getThreads(const std::string& threadName)
{
    std::unique_lock<std::mutex> lock(m_mutex);
    std::vector<Thread*> result;

    // iterate over names
    const auto it = m_allThreads.find(threadName);
    if (it != m_allThreads.end()) {
        // iterate over ids
        for (const auto& [id, thread] : it->second) {
            result.push_back(thread);
        }
    }

    return result;
}

/**
 * @brief request a new id
 *
 * @return new id
 */
uint64_t
ThreadHandler::getNewId()
{
    std::unique_lock<std::mutex> lock(m_mutex);

    m_instance->m_counter++;
    return m_counter;
}

/**
 * @brief add thread to thread-handler
 *
 * @param thread pointer to the new thread to register
 */
void
ThreadHandler::registerThread(Thread* thread)
{
    std::unique_lock<std::mutex> lock(m_mutex);

    // if name is already registered then add this to the name
    auto it = m_allThreads.find(thread->getThreadName());
    if (it != m_allThreads.end()) {
        it->second.insert(std::make_pair(thread->getThreadId(), thread));
        return;
    }

    // if name is not even registered, ass a complete new entry
    std::map<uint64_t, Thread*> newEntry;
    newEntry.try_emplace(thread->getThreadId(), thread);
    m_allThreads.try_emplace(thread->getThreadName(), newEntry);

    return;
}

/**
 * @brief remove thread from thead-handler
 *
 * @param threadName name of the group of the thread
 * @param threadId id of the thread
 *
 * @return true, if found and unregistered, else false
 */
bool
ThreadHandler::unregisterThread(const std::string& threadName, const uint64_t threadId)
{
    std::unique_lock<std::mutex> lock(m_mutex);
    bool found = false;

    // iterate over names
    auto name_it = m_allThreads.find(threadName);
    if (name_it != m_allThreads.end()) {
        // iterate over ids
        auto id_it = name_it->second.find(threadId);
        if (id_it != name_it->second.end()) {
            name_it->second.erase(id_it);
            found = true;
        }

        // if there are no more threads with the name, then remove the whole entry
        // to avoid a memory-leak
        if (name_it->second.size() == 0) {
            m_allThreads.erase(name_it);
        }
    }

    return found;
}

}  // namespace Hanami
