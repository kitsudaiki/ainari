#include <hanami_common/buffer/stack_buffer_reserve.h>

namespace Hanami
{

StackBufferReserve* StackBufferReserve::m_stackBufferReserve = new StackBufferReserve();

/**
 * @brief constructor
 *
 * @param reserveSize maximum number of items in reserver
 */
StackBufferReserve::StackBufferReserve(const uint32_t reserveSize)
{
    assert(STACK_BUFFER_BLOCK_SIZE % 4096 == 0);

    m_reserveSize = reserveSize;
}

/**
 * @brief destructor
 */
StackBufferReserve::~StackBufferReserve()
{
    while (m_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    // delete all buffer within the reserve and free the memory
    for (uint64_t i = 0; i < m_reserve.size(); i++) {
        DataBuffer* temp = m_reserve.at(i);
        delete temp;
    }

    m_lock.clear(std::memory_order_release);
}

/**
 * @brief static methode to get instance of the interface
 *
 * @return pointer to the static instance
 */
StackBufferReserve*
StackBufferReserve::getInstance()
{
    return m_stackBufferReserve;
}

/**
 * @brief add buffer to the reserve
 *
 * @param buffer data-buffer-pointer to add
 *
 * @return false, if buffer was nullptr, else true
 */
bool
StackBufferReserve::addBuffer(DataBuffer* buffer)
{
    // precheck
    if (buffer == nullptr) {
        return false;
    }

    while (m_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    if (m_reserve.size() >= m_reserveSize) {
        // delete given buffer, if there are already too much within the reserve
        delete buffer;
    }
    else {
        // reset buffer and add to reserve
        buffer->usedBufferSize = 0;
        m_reserve.push_back(buffer);
    }

    m_lock.clear(std::memory_order_release);

    return true;
}

/**
 * @brief get number of buffer within the reserve
 *
 * @return number of data-buffer
 */
uint64_t
StackBufferReserve::getNumberOfBuffers()
{
    while (m_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    const uint64_t result = m_reserve.size();

    m_lock.clear(std::memory_order_release);

    return result;
}

/**
 * @brief get data-buffer from the reserve
 *
 * @return pointer to the data-buffer
 */
DataBuffer*
StackBufferReserve::getBuffer()
{
    while (m_lock.test_and_set(std::memory_order_acquire)) {
        asm("");
    }

    if (m_reserve.size() == 0) {
        m_lock.clear(std::memory_order_release);
        return new DataBuffer(STACK_BUFFER_BLOCK_SIZE / 4096, 4096);
    }

    DataBuffer* result = m_reserve.back();
    m_reserve.pop_back();

    m_lock.clear(std::memory_order_release);

    return result;
}

}  // namespace Hanami
