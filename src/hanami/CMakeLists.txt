cmake_minimum_required(VERSION 3.16)

project(hanami CXX)
# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# project(hanami CXX CUDA)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Adding source files
set(SOURCES
    src/api/endpoint_processing/auth_check.h
    src/api/endpoint_processing/blossom.h
    src/api/endpoint_processing/http_processing/http_processing.h
    src/api/endpoint_processing/http_processing/response_builds.h
    src/api/endpoint_processing/http_processing/string_functions.h
    src/api/endpoint_processing/http_server.h
    src/api/endpoint_processing/http_websocket_thread.h
    src/api/endpoint_processing/runtime_validation.h
    src/api/http/auth/create_token_v1_0.h
    src/api/http/auth/renew_token_v1_0.h
    src/api/http/checkpoint/delete_checkpoint_v1_0.h
    src/api/http/checkpoint/get_checkpoint_v1_0.h
    src/api/http/checkpoint/list_checkpoint_v1_0.h
    src/api/http/cluster/create_cluster_v1_0.h
    src/api/http/cluster/delete_cluster_v1_0.h
    src/api/http/cluster/list_cluster_v1_0.h
    src/api/http/cluster/load_cluster_v1_0.h
    src/api/http/cluster/save_cluster_v1_0.h
    src/api/http/cluster/set_cluster_mode_v1_0.h
    src/api/http/cluster/show_cluster_v1_0.h
    src/api/http/cluster/switch_hosts_v1_0.h
    src/api/http/dataset/check_dataset_v1_0.h
    src/api/http/dataset/csv/create_csv_dataset_v1_0.h
    src/api/http/dataset/csv/finalize_csv_dataset_v1_0.h
    src/api/http/dataset/delete_dataset_v1_0.h
    src/api/http/dataset/download_dataset_content_v1_0.h
    src/api/http/dataset/get_dataset_v1_0.h
    src/api/http/dataset/get_progress_dataset_v1_0.h
    src/api/http/dataset/list_dataset_v1_0.h
    src/api/http/dataset/mnist/create_mnist_dataset_v1_0.h
    src/api/http/dataset/mnist/finalize_mnist_dataset_v1_0.h
    src/api/http/endpoint_init.h
    src/api/http/hosts/list_hosts_v1_0.h
    src/api/http/logs/get_audit_log_v1_0.h
    src/api/http/logs/get_error_log_v1_0.h
    src/api/http/measurements/power_consumption_v1_0.h
    src/api/http/measurements/speed_v1_0.h
    src/api/http/measurements/temperature_production_v1_0.h
    src/api/http/project/create_project_v1_0.h
    src/api/http/project/delete_project_v1_0.h
    src/api/http/project/get_project_v1_0.h
    src/api/http/project/list_projects_v1_0.h
    src/api/http/system_info/get_system_info_v1_0.h
    src/api/http/task/create_request_task_v1_0.h
    src/api/http/task/create_train_task_v1_0.h
    src/api/http/task/delete_task_v1_0.h
    src/api/http/task/list_task_v1_0.h
    src/api/http/task/show_task_v1_0.h
    src/api/http/threading/get_thread_mapping_v1_0.h
    src/api/http/user/add_project_to_user_v1_0.h
    src/api/http/user/create_user_v1_0.h
    src/api/http/user/delete_user_v1_0.h
    src/api/http/user/get_user_v1_0.h
    src/api/http/user/list_user_projects_v1_0.h
    src/api/http/user/list_users_v1_0.h
    src/api/http/user/remove_project_from_user_v1_0.h
    src/api/websocket/cluster_io.h
    src/api/websocket/file_upload.h
    src/args.h
    src/config.h
    src/core/cluster/cluster.h
    src/core/cluster/cluster_handler.h
    src/core/cluster/cluster_init.h
    src/core/cluster/cluster_io_convert.h
    src/core/cluster/objects.h
    src/core/cluster/statemachine_init.h
    src/core/cluster/states/cycle_finish_state.h
    src/core/cluster/states/request_state.h
    src/core/cluster/states/restore_cluster_state.h
    src/core/cluster/states/save_cluster_state.h
    src/core/cluster/states/task_handle_state.h
    src/core/cluster/states/train_forward_state.h
    src/core/cluster/task.h
    src/core/io/checkpoint/buffer/buffer_io.h
    src/core/io/checkpoint/disc/checkpoint_io.h
    src/core/io/checkpoint/io_interface.h
    src/core/io/data_set/dataset_file_io.h
    src/core/processing/cluster_resize.h
    src/core/processing/cpu/backpropagation.h
    src/core/processing/cpu/cpu_host.h
    src/core/processing/cpu/cpu_worker_thread.h
    src/core/processing/cpu/processing.h
    src/core/processing/cpu/reduction.h
    src/core/processing/cuda/cuda_functions.h
    src/core/processing/cuda/cuda_host.h
    src/core/processing/cuda/cuda_worker_thread.h
    src/core/processing/cuda/error_handling.h
    src/core/processing/cuda/info.h
    src/core/processing/logical_host.h
    src/core/processing/physical_host.h
    src/core/processing/worker_thread.h
    src/core/routing_functions.h
    src/core/temp_file_handler.h
    src/core/thread_binder.h
    src/database/audit_log_table.h
    src/database/checkpoint_table.h
    src/database/cluster_table.h
    src/database/dataset_table.h
    src/database/error_log_table.h
    src/database/generic_tables/hanami_sql_admin_table.h
    src/database/generic_tables/hanami_sql_log_table.h
    src/database/generic_tables/hanami_sql_table.h
    src/database/projects_table.h
    src/database/tempfile_table.h
    src/database/users_table.h
    src/documentation/generate_rest_api_docu.h
    src/hanami_root.h
    src/api/endpoint_processing/auth_check.cpp
    src/api/endpoint_processing/blossom.cpp
    src/api/endpoint_processing/http_processing/http_processing.cpp
    src/api/endpoint_processing/http_server.cpp
    src/api/endpoint_processing/http_websocket_thread.cpp
    src/api/endpoint_processing/runtime_validation.cpp
    src/api/http/auth/create_token_v1_0.cpp
    src/api/http/auth/renew_token_v1_0.cpp
    src/api/http/checkpoint/delete_checkpoint_v1_0.cpp
    src/api/http/checkpoint/get_checkpoint_v1_0.cpp
    src/api/http/checkpoint/list_checkpoint_v1_0.cpp
    src/api/http/cluster/create_cluster_v1_0.cpp
    src/api/http/cluster/delete_cluster_v1_0.cpp
    src/api/http/cluster/list_cluster_v1_0.cpp
    src/api/http/cluster/load_cluster_v1_0.cpp
    src/api/http/cluster/save_cluster_v1_0.cpp
    src/api/http/cluster/set_cluster_mode_v1_0.cpp
    src/api/http/cluster/show_cluster_v1_0.cpp
    src/api/http/cluster/switch_hosts_v1_0.cpp
    src/api/http/dataset/check_dataset_v1_0.cpp
    src/api/http/dataset/csv/create_csv_dataset_v1_0.cpp
    src/api/http/dataset/csv/finalize_csv_dataset_v1_0.cpp
    src/api/http/dataset/delete_dataset_v1_0.cpp
    src/api/http/dataset/download_dataset_content_v1_0.cpp
    src/api/http/dataset/get_dataset_v1_0.cpp
    src/api/http/dataset/get_progress_dataset_v1_0.cpp
    src/api/http/dataset/list_dataset_v1_0.cpp
    src/api/http/dataset/mnist/create_mnist_dataset_v1_0.cpp
    src/api/http/dataset/mnist/finalize_mnist_dataset_v1_0.cpp
    src/api/http/endpoint_init.cpp
    src/api/http/hosts/list_hosts_v1_0.cpp
    src/api/http/logs/get_audit_log_v1_0.cpp
    src/api/http/logs/get_error_log_v1_0.cpp
    src/api/http/measurements/power_consumption_v1_0.cpp
    src/api/http/measurements/speed_v1_0.cpp
    src/api/http/measurements/temperature_production_v1_0.cpp
    src/api/http/project/create_project_v1_0.cpp
    src/api/http/project/delete_project_v1_0.cpp
    src/api/http/project/get_project_v1_0.cpp
    src/api/http/project/list_projects_v1_0.cpp
    src/api/http/system_info/get_system_info_v1_0.cpp
    src/api/http/task/create_request_task_v1_0.cpp
    src/api/http/task/create_train_task_v1_0.cpp
    src/api/http/task/delete_task_v1_0.cpp
    src/api/http/task/list_task_v1_0.cpp
    src/api/http/task/show_task_v1_0.cpp
    src/api/http/threading/get_thread_mapping_v1_0.cpp
    src/api/http/user/add_project_to_user_v1_0.cpp
    src/api/http/user/create_user_v1_0.cpp
    src/api/http/user/delete_user_v1_0.cpp
    src/api/http/user/get_user_v1_0.cpp
    src/api/http/user/list_user_projects_v1_0.cpp
    src/api/http/user/list_users_v1_0.cpp
    src/api/http/user/remove_project_from_user_v1_0.cpp
    src/api/websocket/cluster_io.cpp
    src/api/websocket/file_upload.cpp
    src/core/cluster/cluster.cpp
    src/core/cluster/cluster_handler.cpp
    src/core/cluster/cluster_init.cpp
    src/core/cluster/cluster_io_convert.cpp
    src/core/cluster/statemachine_init.cpp
    src/core/cluster/states/cycle_finish_state.cpp
    src/core/cluster/states/request_state.cpp
    src/core/cluster/states/restore_cluster_state.cpp
    src/core/cluster/states/save_cluster_state.cpp
    src/core/cluster/states/task_handle_state.cpp
    src/core/cluster/states/train_forward_state.cpp
    src/core/io/checkpoint/buffer/buffer_io.cpp
    src/core/io/checkpoint/disc/checkpoint_io.cpp
    src/core/io/checkpoint/io_interface.cpp
    src/core/io/data_set/dataset_file_io.cpp
    src/core/processing/cpu/cpu_host.cpp
    src/core/processing/cpu/cpu_worker_thread.cpp
    src/core/processing/cuda/cuda_host.cpp
    src/core/processing/cuda/cuda_worker_thread.cpp
    src/core/processing/cuda/info.cpp
    src/core/processing/logical_host.cpp
    src/core/processing/physical_host.cpp
    src/core/processing/worker_thread.cpp
    src/core/temp_file_handler.cpp
    src/core/thread_binder.cpp
    src/database/audit_log_table.cpp
    src/database/checkpoint_table.cpp
    src/database/cluster_table.cpp
    src/database/dataset_table.cpp
    src/database/error_log_table.cpp
    src/database/generic_tables/hanami_sql_admin_table.cpp
    src/database/generic_tables/hanami_sql_log_table.cpp
    src/database/generic_tables/hanami_sql_table.cpp
    src/database/projects_table.cpp
    src/database/tempfile_table.cpp
    src/database/users_table.cpp
    src/documentation/generate_rest_api_docu.cpp
    src/hanami_root.cpp
)

# Adding the executable
add_executable(hanami
    ${SOURCES}
    src/main.cpp

    # TODO:enable gpu init here again
    # related issue: https://github.com/kitsudaiki/Hanami/issues/325
    # ${CUDA_OUTPUT_OBJ}
)
if(run_tests)
    add_executable(hanami_unittests
        ${SOURCES}
        tests/unit_tests/core/cluster_io_convert_test.cpp
        tests/unit_tests/core/cluster_io_convert_test.h
        tests/unit_tests/core/cluster_test.cpp
        tests/unit_tests/core/cluster_test.h
        tests/unit_tests/core/dataset_io_test.cpp
        tests/unit_tests/core/dataset_io_test.h
        tests/unit_tests/core/processing_test.cpp
        tests/unit_tests/core/processing_test.h
        tests/unit_tests/database/audit_log_table_test.cpp
        tests/unit_tests/database/audit_log_table_test.h
        tests/unit_tests/database/checkpoint_table_test.cpp
        tests/unit_tests/database/checkpoint_table_test.h
        tests/unit_tests/database/cluster_table_test.cpp
        tests/unit_tests/database/cluster_table_test.h
        tests/unit_tests/database/dataset_table_test.cpp
        tests/unit_tests/database/dataset_table_test.h
        tests/unit_tests/database/error_log_table_test.cpp
        tests/unit_tests/database/error_log_table_test.h
        tests/unit_tests/database/projects_table_test.cpp
        tests/unit_tests/database/projects_table_test.h
        tests/unit_tests/database/tempfile_table_test.cpp
        tests/unit_tests/database/tempfile_table_test.h
        tests/unit_tests/database/users_table_test.cpp
        tests/unit_tests/database/users_table_test.h
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
    ../libraries/hanami_messages/protobuffers
    ../libraries/hanami_cpu/include
    ../libraries/hanami_hardware/include
    ../libraries/hanami_common/include
    ../libraries/hanami_ini/include
    ../libraries/hanami_config/include
    ../libraries/hanami_sqlite/include
    ../libraries/hanami_database/include
    ../libraries/hanami_crypto/include
    ../libraries/hanami_args/include
    ../libraries/hanami_policies/include
    ../libraries/hanami_cluster_parser/include
    ../third-party-libs/jwt-cpp/include
    ../third-party-libs/json/include
)

# Finding required packages or libraries
find_package(OpenSSL REQUIRED)
find_package(Threads REQUIRED)
find_library(SSL_LIB ssl)
find_library(CRYPTO_LIB crypto)
find_library(SQLITE3_LIB sqlite3)
find_library(UUID_LIB uuid)
find_library(PROTOBUF_LIB protobuf)
# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# find_package(CUDA REQUIRED)
# find_library(CUDA_LIB cuda)
# find_library(CUDART_LIB cudart)

# Linking libraries to the executable
target_link_libraries(hanami
    hanami_cpu
    hanami_hardware
    hanami_common
    hanami_ini
    hanami_config
    hanami_sqlite
    hanami_database
    hanami_crypto
    hanami_args
    hanami_policies
    hanami_cluster_parser
    ${SSL_LIB}
    ${CRYPTO_LIB}
    ${SQLITE3_LIB}
    ${UUID_LIB}
    ${PROTOBUF_LIB}
    Threads::Threads
    OpenSSL::SSL
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
        hanami_ini
        hanami_config
        hanami_sqlite
        hanami_database
        hanami_crypto
        hanami_args
        hanami_policies
        hanami_cluster_parser
        ${SSL_LIB}
        ${CRYPTO_LIB}
        ${SQLITE3_LIB}
        ${UUID_LIB}
        ${PROTOBUF_LIB}
        Threads::Threads
        OpenSSL::SSL
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_LIB}
        # ${CUDART_LIB}
    )
    target_link_libraries(hanami_memory_leak_tests
        hanami_cpu
        hanami_hardware
        hanami_common
        hanami_ini
        hanami_config
        hanami_sqlite
        hanami_database
        hanami_crypto
        hanami_args
        hanami_policies
        hanami_cluster_parser
        ${SSL_LIB}
        ${CRYPTO_LIB}
        ${SQLITE3_LIB}
        ${UUID_LIB}
        ${PROTOBUF_LIB}
        Threads::Threads
        OpenSSL::SSL
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_LIB}
        # ${CUDART_LIB}
    )
endif()

# Set the path to your .proto file
set(PROTO_FILE ../libraries/hanami_messages/protobuffers)

# Set the output directory for generated files
set(PROTO_OUTPUT_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(PROTO_OUTPUT_HEADER ${PROTO_OUTPUT_DIR}/${PROTO_FILE}/hanami_messages.proto3.pb.h)
set(PROTO_OUTPUT_SOURCE ${PROTO_OUTPUT_DIR}/${PROTO_FILE}/hanami_messages.proto3.pb.cc)
# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# set(CUDA_IO_OBJ ${CMAKE_CURRENT_BINARY_DIR}/cuda_io.o)

# TODO:enable gpu init here again
# related issue: https://github.com/kitsudaiki/Hanami/issues/325
# set(CUDA_PROCESSING_OBJ ${CMAKE_CURRENT_BINARY_DIR}/processing.o)
# set(CUDA_BACKPROPAGATION_OBJ ${CMAKE_CURRENT_BINARY_DIR}/backpropagation.o)
# set(CUDA_REDUCTION_OBJ ${CMAKE_CURRENT_BINARY_DIR}/reduction.o)

# Generate C++ headers and sources from the .proto file
add_custom_command(
    OUTPUT
        ${PROTO_OUTPUT_HEADER}
        ${PROTO_OUTPUT_SOURCE}
    COMMAND protoc
        --cpp_out=${PROTO_OUTPUT_DIR}/${PROTO_FILE}
        --proto_path=${CMAKE_CURRENT_SOURCE_DIR}/${PROTO_FILE}
        ${CMAKE_CURRENT_SOURCE_DIR}/${PROTO_FILE}/hanami_messages.proto3
    DEPENDS
        ${CMAKE_CURRENT_SOURCE_DIR}/${PROTO_FILE}/hanami_messages.proto3
)

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

# Add dependencies on the targets representing the generated files
add_custom_target(generate_protobuf_files DEPENDS
    ${PROTO_OUTPUT_HEADER}
    ${PROTO_OUTPUT_SOURCE}
)
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

target_sources(hanami PRIVATE
    ${PROTO_OUTPUT_HEADER}
    ${PROTO_OUTPUT_SOURCE}
    # TODO:enable gpu init here again
    # related issue: https://github.com/kitsudaiki/Hanami/issues/325
    # ${CUDA_IO_OBJ}
    # ${CUDA_PROCESSING_OBJ}
    # ${CUDA_BACKPROPAGATION_OBJ}
    # ${CUDA_REDUCTION_OBJ}
)
if(run_tests)
    # TODO:enable gpu init here again
    # related issue: https://github.com/kitsudaiki/Hanami/issues/325
    # add_dependencies(hanami_unittests compile_cuda_kernel)
    add_dependencies(hanami_unittests generate_protobuf_files)
    target_sources(hanami_unittests PRIVATE
        ${PROTO_OUTPUT_HEADER}
        ${PROTO_OUTPUT_SOURCE}
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_IO_OBJ}
        # ${CUDA_PROCESSING_OBJ}
        # ${CUDA_BACKPROPAGATION_OBJ}
        # ${CUDA_REDUCTION_OBJ}
    )
    add_dependencies(hanami_memory_leak_tests generate_protobuf_files)
    target_sources(hanami_memory_leak_tests PRIVATE
        ${PROTO_OUTPUT_HEADER}
        ${PROTO_OUTPUT_SOURCE}
        # TODO:enable gpu init here again
        # related issue: https://github.com/kitsudaiki/Hanami/issues/325
        # ${CUDA_IO_OBJ}
        # ${CUDA_PROCESSING_OBJ}
        # ${CUDA_BACKPROPAGATION_OBJ}
        # ${CUDA_REDUCTION_OBJ}
    )
endif()

