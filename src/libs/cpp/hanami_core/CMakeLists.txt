cmake_minimum_required(VERSION 3.16)
project(hanami_core VERSION 0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(hanami_core STATIC
    include/hanami_core/hanami_root.h
    src/cluster/cluster.h
    src/cluster/cluster_handler.h
    src/cluster/cluster_init.h
    src/cluster/cluster_io_convert.h
    src/cluster/objects.h
    src/cluster/statemachine_init.h
    src/cluster/states/cycle_finish_state.h
    src/cluster/states/request_state.h
    src/cluster/states/task_handle_state.h
    src/cluster/states/train_forward_state.h
    src/cluster/task.h
    src/io/checkpoint/buffer/buffer_io.h
    src/io/checkpoint/disc/checkpoint_io.h
    src/io/checkpoint/io_interface.h
    src/io/data_set/dataset_file_io.h
    src/processing/axon_handling.h
    src/processing/cluster_resize.h
    src/processing/cpu/core_backpropagation.h
    src/processing/cpu/output_backpropagation.h
    src/processing/cpu/cpu_host.h
    src/processing/cpu/cpu_worker_thread.h
    src/processing/cpu/core_processing.h
    src/processing/cpu/output_processing.h
    src/processing/cuda/cuda_functions.h
    src/processing/cuda/cuda_host.h
    src/processing/cuda/cuda_worker_thread.h
    src/processing/cuda/error_handling.h
    src/processing/cuda/info.h
    src/processing/logical_host.h
    src/processing/physical_host.h
    src/processing/worker_thread.h
    src/routing_functions.h
    src/thread_binder.h
    src/cluster/cluster.cpp
    src/cluster/cluster_handler.cpp
    src/cluster/cluster_init.cpp
    src/cluster/cluster_io_convert.cpp
    src/cluster/statemachine_init.cpp
    src/cluster/states/cycle_finish_state.cpp
    src/cluster/states/request_state.cpp
    src/cluster/states/task_handle_state.cpp
    src/cluster/states/train_forward_state.cpp
    src/io/checkpoint/buffer/buffer_io.cpp
    src/io/checkpoint/disc/checkpoint_io.cpp
    src/io/checkpoint/io_interface.cpp
    src/io/data_set/dataset_file_io.cpp
    src/processing/cpu/cpu_host.cpp
    src/processing/cpu/cpu_worker_thread.cpp
    src/processing/cuda/cuda_host.cpp
    src/processing/cuda/cuda_worker_thread.cpp
    src/processing/cuda/info.cpp
    src/processing/logical_host.cpp
    src/processing/physical_host.cpp
    src/processing/worker_thread.cpp
    src/thread_binder.cpp
    src/hanami_root.cpp
    tests/unit_tests/core/hanami_core_test.h tests/unit_tests/core/hanami_core_test.cpp
    include/hanami_core/types.h
)

if(run_tests)
    add_executable(hanami_unittests
        ${SOURCES}
        tests/unit_tests/core/hanami_core_test.cpp
        tests/unit_tests/core/hanami_core_test.h
        tests/unit_tests/core/cluster_io_convert_test.cpp
        tests/unit_tests/core/cluster_io_convert_test.h
        tests/unit_tests/core/cluster_test.cpp
        tests/unit_tests/core/cluster_test.h
        tests/unit_tests/core/dataset_io_test.cpp
        tests/unit_tests/core/dataset_io_test.h
        tests/unit_tests/core/processing_test.cpp
        tests/unit_tests/core/processing_test.h
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
        tests/memory_leak_tests/core/dataset_io_test.cpp
        tests/memory_leak_tests/core/dataset_io_test.h
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
    src/
    ../libraries/hanami_cpu/include
    ../libraries/hanami_hardware/include
    ../libraries/hanami_common/include
    ../libraries/hanami_crypto/include
)

# This hard-coded output-path is necessary for easier development of the rust-library
# so it can easily find the shared library, to avoid stupid linking problems
#set_target_properties(hanami_core PROPERTIES
#    LIBRARY_OUTPUT_DIRECTORY_DEBUG "/tmp/hanami_core/debug"
#    LIBRARY_OUTPUT_DIRECTORY_RELEASE "/tmp/hanami_core/release"
#)

# Finding required packages or libraries
find_package(Threads REQUIRED)
# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# find_package(CUDA REQUIRED)
# find_library(CUDA_LIB cuda)
# find_library(CUDART_LIB cudart)

# Linking libraries to the executable
target_link_libraries(hanami_core
    hanami_cpu
    hanami_hardware
    hanami_common
    Threads::Threads
    # TODO:enable gpu init here again
    # related issue: https://github.com/kitsudaiki/Hanami/issues/325
    # ${CUDA_LIB}
    # ${CUDART_LIB}
)
if(run_tests)

    target_link_libraries(hanami_unittests
        hanami_cpu
        hanami_hardware
        hanami_common
        hanami_core
        Threads::Threads
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_LIB}
        # ${CUDART_LIB}
    )
    target_link_libraries(hanami_memory_leak_tests
        hanami_cpu
        hanami_hardware
        hanami_common
        hanami_core
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
#        -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/hanami_common/include
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/processing.cu
#        -o ${CUDA_PROCESSING_OBJ}
#    COMMAND nvcc
#        -O3
#        -c
#        -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/hanami_common/include
#        ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/backpropagation.cu
#        -o ${CUDA_BACKPROPAGATION_OBJ}
#    #COMMAND nvcc
#    #    -O3
#    #    -c
#    #    -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/hanami_common/include
#    #    ${CMAKE_CURRENT_SOURCE_DIR}/src/core/processing/cuda/reduction.cu
#    #    -o ${CUDA_REDUCTION_OBJ}

#    COMMAND nvcc
#        -O3
#        -c
#        -I${CMAKE_CURRENT_SOURCE_DIR}/../libraries/hanami_common/include
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
