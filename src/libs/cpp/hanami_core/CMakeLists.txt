cmake_minimum_required(VERSION 3.16)
project(hanami_core_cpp VERSION 0.1 LANGUAGES C CXX)


set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)


add_library(hanami_core_cpp STATIC
    hanami_root.h
    hanami_structs.h
    cluster_link.h
    src/cluster/cluster_link.cpp
    src/cluster/cluster.h
    src/cluster/objects.h
    src/cluster/cluster.cpp
    src/cluster/cluster_handler.cpp
    src/cluster/cluster_handler.h
    src/cluster/cluster_init.cpp
    src/cluster/cluster_init.h
    src/cluster/cluster_io_convert.cpp
    src/cluster/cluster_io_convert.h
    src/common/defines.h
    src/common/structs.h
    src/common/types.h
    src/common/enums.h
    src/common/buffer/data_buffer.h
    src/common/buffer/item_buffer.h
    src/common/files/binary_file.cpp
    src/common/files/binary_file.h
    src/common/files/binary_file_direct.cpp
    src/common/files/binary_file_direct.h
    src/common/functions/common_functions.h
    src/common/functions/file_functions.cpp
    src/common/functions/file_functions.h
    src/common/functions/string_functions.cpp
    src/common/functions/string_functions.h
    src/common/functions/time_functions.h
    src/common/functions/vector_functions.cpp
    src/common/functions/vector_functions.h
    src/common/items/table_item.cpp
    src/common/items/table_item.h
    src/common/logger.cpp
    src/common/logger.h
    src/common/memory_counter.cpp
    src/common/memory_counter.h
    src/common/process_execution.cpp
    src/common/process_execution.h
    src/common/test_helper/compare_test_helper.cpp
    src/common/test_helper/compare_test_helper.h
    src/common/test_helper/memory_leak_test_helper.cpp
    src/common/test_helper/memory_leak_test_helper.h
    src/common/test_helper/speed_test_helper.cpp
    src/common/test_helper/speed_test_helper.h
    src/common/threading/barrier.cpp
    src/common/threading/barrier.h
    src/common/threading/cleanup_thread.cpp
    src/common/threading/cleanup_thread.h
    src/common/threading/thread.cpp
    src/common/threading/thread.h
    src/common/threading/thread_handler.cpp
    src/common/threading/thread_handler.h
    src/hanami_root.cpp
    src/hardware/cpu.cpp
    src/hardware/cpu.h
    src/hardware/cpu_core.cpp
    src/hardware/cpu_core.h
    src/hardware/cpu_package.cpp
    src/hardware/cpu_package.h
    src/hardware/cpu_thread.cpp
    src/hardware/cpu_thread.h
    src/hardware/host.cpp
    src/hardware/host.h
    src/hardware/memory.cpp
    src/hardware/memory.h
    src/hardware/power_measuring.cpp
    src/hardware/power_measuring.h
    src/hardware/rapl.cpp
    src/hardware/rapl.h
    src/hardware/speed_measuring.cpp
    src/hardware/speed_measuring.h
    src/hardware/temperature_measuring.cpp
    src/hardware/temperature_measuring.h
    src/hardware/value_container.cpp
    src/hardware/value_container.h
    src/io/checkpoint/buffer/buffer_io.cpp
    src/io/checkpoint/buffer/buffer_io.h
    src/io/checkpoint/disc/checkpoint_io.cpp
    src/io/checkpoint/disc/checkpoint_io.h
    src/io/checkpoint/io_interface.cpp
    src/io/checkpoint/io_interface.h
    src/processing/axon_handling.h
    src/processing/cluster_resize.h
    src/processing/cpu/core_backpropagation.h
    src/processing/cpu/core_processing.h
    src/processing/cpu/cpu_host.cpp
    src/processing/cpu/cpu_host.h
    src/processing/cpu/cpu_worker_thread.cpp
    src/processing/cpu/cpu_worker_thread.h
    src/processing/cpu/output_backpropagation.h
    src/processing/cpu/output_processing.h
    src/processing/cuda/cuda_functions.h
    src/processing/cuda/cuda_host.cpp
    src/processing/cuda/cuda_host.h
    src/processing/cuda/cuda_worker_thread.cpp
    src/processing/cuda/cuda_worker_thread.h
    src/processing/cuda/error_handling.h
    src/processing/cuda/info.cpp
    src/processing/cuda/info.h
    src/processing/logical_host.cpp
    src/processing/logical_host.h
    src/processing/physical_host.cpp
    src/processing/physical_host.h
    src/processing/worker_thread.cpp
    src/processing/worker_thread.h
    src/routing_functions.h
    src/thread_binder.cpp
    src/thread_binder.h
    tests/unit_tests/core/hanami_core_test.h tests/unit_tests/core/hanami_core_test.cpp
)

target_include_directories(hanami_core_cpp PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
install(FILES hanami_root.h hanami_structs.h cluster_link.h DESTINATION include)

if(run_tests)
    add_executable(hanami_unittests
        ${SOURCES}
        tests/unit_tests/core/hanami_core_test.cpp
        tests/unit_tests/core/hanami_core_test.h
        tests/unit_tests/core/cluster_io_convert_test.cpp
        tests/unit_tests/core/cluster_io_convert_test.h
        tests/unit_tests/core/cluster_test.cpp
        tests/unit_tests/core/cluster_test.h
        tests/unit_tests/core/processing_test.cpp
        tests/unit_tests/core/processing_test.h
        tests/unit_tests/common/buffer/data_buffer_test.cpp
        tests/unit_tests/common/buffer/data_buffer_test.h
        tests/unit_tests/common/buffer/item_buffer_test.cpp
        tests/unit_tests/common/buffer/item_buffer_test.h
        tests/unit_tests/common/files/binary_file_with_directIO_test.cpp
        tests/unit_tests/common/files/binary_file_with_directIO_test.h
        tests/unit_tests/common/files/binary_file_without_directIO_test.cpp
        tests/unit_tests/common/files/binary_file_without_directIO_test.h
        tests/unit_tests/common/items/table_item_test.cpp
        tests/unit_tests/common/items/table_item_test.h
        tests/unit_tests/common/logger_test.cpp
        tests/unit_tests/common/logger_test.h
        tests/unit_tests/common/functions/file_functions_test.cpp
        tests/unit_tests/common/functions/file_functions_test.h
        tests/unit_tests/common/functions/string_functions_test.cpp
        tests/unit_tests/common/functions/string_functions_test.h
        tests/unit_tests/common/functions/vector_functions_test.cpp
        tests/unit_tests/common/functions/vector_functions_test.h
        tests/unit_tests/common/threading/thread_handler_test.cpp
        tests/unit_tests/common/threading/thread_handler_test.h
        tests/unit_tests/main.cpp


        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_IO_OBJ}
        # ${CUDA_PROCESSING_OBJ}
        # ${CUDA_BACKPROPAGATION_OBJ}
        # ${CUDA_REDUCTION_OBJ}
    )
    add_executable(hanami_memory_leak_tests
        ${SOURCES}
        tests/memory_leak_tests/core/cluster_test.cpp
        tests/memory_leak_tests/core/cluster_test.h
        tests/memory_leak_tests/common/buffer/data_buffer_test.cpp
        tests/memory_leak_tests/common/buffer/data_buffer_test.h
        tests/memory_leak_tests/common/buffer/item_buffer_test.cpp
        tests/memory_leak_tests/common/buffer/item_buffer_test.h
        tests/memory_leak_tests/common/items/table_item_test.cpp
        tests/memory_leak_tests/common/items/table_item_test.h


        tests/memory_leak_tests/common/threading/bogus_thread.cpp
        tests/memory_leak_tests/common/threading/bogus_thread.h
        tests/memory_leak_tests/common/threading/thread_test.cpp
        tests/memory_leak_tests/common/threading/thread_test.h
        tests/memory_leak_tests/main.cpp

        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_IO_OBJ}
        # ${CUDA_PROCESSING_OBJ}
        # ${CUDA_BACKPROPAGATION_OBJ}
        # ${CUDA_REDUCTION_OBJ}
    )

endif()

# Including directories
include_directories(
    .
    include/hanami_core/
    src/
)

# Finding required packages or libraries
find_package(Threads REQUIRED)
# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# find_package(CUDA REQUIRED)
# find_library(CUDA_LIB cuda)
# find_library(CUDART_LIB cudart)

# Linking libraries to the executable
target_link_libraries(hanami_core_cpp
    Threads::Threads
    # TODO:enable gpu init here again
    # related issue: https://github.com/kitsudaiki/Hanami/issues/325
    # ${CUDA_LIB}
    # ${CUDART_LIB}
)
if(run_tests)

    target_link_libraries(hanami_unittests
        hanami_core_cpp
        Threads::Threads
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_LIB}
        # ${CUDART_LIB}
    )
    target_link_libraries(hanami_memory_leak_tests
        hanami_core_cpp
        Threads::Threads
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_LIB}
        # ${CUDART_LIB}
    )
endif()


# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# set(CUDA_IO_OBJ ${CMAKE_CURRENT_BINARY_DIR}/cuda_io.o)

# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# set(CUDA_PROCESSING_OBJ ${CMAKE_CURRENT_BINARY_DIR}/processing.o)
# set(CUDA_BACKPROPAGATION_OBJ ${CMAKE_CURRENT_BINARY_DIR}/backpropagation.o)
# set(CUDA_REDUCTION_OBJ ${CMAKE_CURRENT_BINARY_DIR}/reduction.o)


# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
#add_custom_command(
#    OUTPUT
#        ${CUDA_IO_OBJ}
#        ${CUDA_PROCESSING_OBJ}
#        ${CUDA_BACKPROPAGATION_OBJ}
#        ${CUDA_REDUCTION_OBJ}
#    COMMAND nvcc
#        -O3
#        -c
#        -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/common/include
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/processing.cu
#        -o ${CUDA_PROCESSING_OBJ}
#    COMMAND nvcc
#        -O3
#        -c
#        -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/common/include
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/backpropagation.cu
#        -o ${CUDA_BACKPROPAGATION_OBJ}
#    #COMMAND nvcc
#    #    -O3
#    #    -c
#    #    -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/common/include
#    #    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/reduction.cu
#    #    -o ${CUDA_REDUCTION_OBJ}

#    COMMAND nvcc
#        -O3
#        -c
#        -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/common/include
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/cuda_io.cu
#        -o ${CUDA_IO_OBJ}

#    DEPENDS
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/cuda_io.cu
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/processing.cu
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/backpropagation.cu
#        #${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/reduction.cu
#)

# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
#add_dependencies(hanami generate_protobuf_files)
#add_custom_target(compile_cuda_kernel DEPENDS
#    ${CUDA_IO_OBJ}
#    ${CUDA_PROCESSING_OBJ}
#    ${CUDA_BACKPROPAGATION_OBJ}
#    # ${CUDA_REDUCTION_OBJ}
#)
#add_dependencies(hanami compile_cuda_kernel)

#target_sources(hanami PRIVATE
    # TODO:enable gpu init here again
    # related issue: https://github.com/kitsudaiki/Hanami/issues/325
    # ${CUDA_IO_OBJ}
    # ${CUDA_PROCESSING_OBJ}
    # ${CUDA_BACKPROPAGATION_OBJ}
    # ${CUDA_REDUCTION_OBJ}
#)
if(run_tests)
    # TODO:enable gpu init here again
    # related issue: https://github.com/kitsudaiki/Hanami/issues/325
    # add_dependencies(hanami_unittests compile_cuda_kernel)
    #add_dependencies(hanami_unittests generate_protobuf_files)
    #target_sources(hanami_unittests PRIVATE
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_IO_OBJ}
        # ${CUDA_PROCESSING_OBJ}
        # ${CUDA_BACKPROPAGATION_OBJ}
        # ${CUDA_REDUCTION_OBJ}
    #)
    #add_dependencies(hanami_memory_leak_tests generate_protobuf_files)
    #target_sources(hanami_memory_leak_tests PRIVATE
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_IO_OBJ}
        # ${CUDA_PROCESSING_OBJ}
        # ${CUDA_BACKPROPAGATION_OBJ}
        # ${CUDA_REDUCTION_OBJ}
    #)
endif()
