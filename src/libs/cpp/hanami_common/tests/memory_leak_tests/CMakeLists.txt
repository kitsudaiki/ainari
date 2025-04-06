add_executable(hanami_common_memory_leak_tests
    hanami_common/buffer/data_buffer_test.cpp
    hanami_common/buffer/data_buffer_test.h
    hanami_common/buffer/item_buffer_test.cpp
    hanami_common/buffer/item_buffer_test.h
    hanami_common/buffer/ring_buffer_test.cpp
    hanami_common/buffer/ring_buffer_test.h
    hanami_common/buffer/stack_buffer_reserve_test.cpp
    hanami_common/buffer/stack_buffer_reserve_test.h
    hanami_common/buffer/stack_buffer_test.cpp
    hanami_common/buffer/stack_buffer_test.h
    hanami_common/items/table_item_test.cpp
    hanami_common/items/table_item_test.h
    hanami_common/state_test.cpp
    hanami_common/state_test.h
    hanami_common/statemachine_test.cpp
    hanami_common/statemachine_test.h
    hanami_common/threading/bogus_event.cpp
    hanami_common/threading/bogus_event.h
    hanami_common/threading/bogus_thread.cpp
    hanami_common/threading/bogus_thread.h
    hanami_common/threading/thread_test.cpp
    hanami_common/threading/thread_test.h
    main.cpp
)

target_include_directories(hanami_common_memory_leak_tests PRIVATE
    include/
    src/
)

target_link_libraries(hanami_common_memory_leak_tests PRIVATE
    hanami_common
)
