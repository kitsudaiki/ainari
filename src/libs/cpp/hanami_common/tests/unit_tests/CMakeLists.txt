add_executable(hanami_common_unit_tests
    hanami_common/buffer/bit_buffer_test.cpp
    hanami_common/buffer/bit_buffer_test.h
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
    hanami_common/files/binary_file_with_directIO_test.cpp 
    hanami_common/files/binary_file_with_directIO_test.h
    hanami_common/files/binary_file_without_directIO_test.cpp 
    hanami_common/files/binary_file_without_directIO_test.h
    hanami_common/files/text_file_test.cpp 
    hanami_common/files/text_file_test.h
    hanami_common/items/table_item_test.cpp 
    hanami_common/items/table_item_test.h
    hanami_common/logger_test.cpp 
    hanami_common/logger_test.h
    hanami_common/functions/file_functions_test.cpp 
    hanami_common/functions/file_functions_test.h
    hanami_common/functions/string_functions_test.cpp 
    hanami_common/functions/string_functions_test.h
    hanami_common/functions/vector_functions_test.cpp 
    hanami_common/functions/vector_functions_test.h
    hanami_common/progress_bar_test.cpp 
    hanami_common/progress_bar_test.h
    hanami_common/state_test.cpp 
    hanami_common/state_test.h
    hanami_common/statemachine_test.cpp 
    hanami_common/statemachine_test.h
    hanami_common/threading/thread_handler_test.cpp 
    hanami_common/threading/thread_handler_test.h
    main.cpp
)

target_include_directories(hanami_common_unit_tests PRIVATE
    ../../include/
    ../../src/
)

target_link_libraries(hanami_common_unit_tests PRIVATE
    hanami_common
)

