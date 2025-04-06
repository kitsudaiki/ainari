cmake_minimum_required(VERSION 3.16)
project(hanami_common VERSION 0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(hanami_common STATIC
    include/hanami_common/defines.h
    include/hanami_common/enums.h
    include/hanami_common/buffer/bit_buffer.h
    include/hanami_common/buffer/data_buffer.h
    include/hanami_common/buffer/item_buffer.h
    include/hanami_common/buffer/ring_buffer.h
    include/hanami_common/buffer/stack_buffer.h
    include/hanami_common/buffer/stack_buffer_reserve.h
    include/hanami_common/files/binary_file.h
    include/hanami_common/files/binary_file_direct.h
    include/hanami_common/files/text_file.h
    include/hanami_common/items/table_item.h
    include/hanami_common/logger.h
    include/hanami_common/memory_counter.h
    include/hanami_common/functions/common_functions.h
    include/hanami_common/functions/file_functions.h
    include/hanami_common/functions/string_functions.h
    include/hanami_common/functions/vector_functions.h
    include/hanami_common/functions/time_functions.h
    include/hanami_common/process_execution.h
    include/hanami_common/progress_bar.h
    include/hanami_common/statemachine.h
    include/hanami_common/structs.h
    include/hanami_common/test_helper/compare_test_helper.h
    include/hanami_common/test_helper/memory_leak_test_helper.h
    include/hanami_common/test_helper/speed_test_helper.h
    include/hanami_common/threading/barrier.h
    include/hanami_common/threading/cleanup_thread.h
    include/hanami_common/threading/event.h
    include/hanami_common/threading/event_queue.h
    include/hanami_common/threading/thread.h
    include/hanami_common/threading/thread_handler.h
    include/hanami_common/upload_file_handle.h
    src/buffer/stack_buffer_reserve.cpp
    src/files/binary_file.cpp
    src/files/binary_file_direct.cpp
    src/files/text_file.cpp
    src/items/table_item.cpp
    src/logger.cpp
    src/memory_counter.cpp
    src/functions/file_functions.cpp
    src/functions/string_functions.cpp
    src/functions/vector_functions.cpp
    src/process_execution.cpp
    src/progress_bar.cpp
    src/state.h
    src/statemachine.cpp
    src/test_helper/compare_test_helper.cpp
    src/test_helper/memory_leak_test_helper.cpp
    src/test_helper/speed_test_helper.cpp
    src/threading/barrier.cpp
    src/threading/cleanup_thread.cpp
    src/threading/event.cpp
    src/threading/event_queue.cpp
    src/threading/thread.cpp
    src/threading/thread_handler.cpp
)

include_directories(
    include/
    src/
)

set_target_properties(hanami_common PROPERTIES POSITION_INDEPENDENT_CODE ON)

if(run_tests)
    add_subdirectory(tests)
endif()
