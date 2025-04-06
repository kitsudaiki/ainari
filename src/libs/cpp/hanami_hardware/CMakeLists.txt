cmake_minimum_required(VERSION 3.16)
project(hanami_hardware VERSION 0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(hanami_hardware STATIC
    include/hanami_hardware/cpu_core.h
    include/hanami_hardware/cpu_package.h
    include/hanami_hardware/cpu_thread.h
    include/hanami_hardware/host.h
    include/hanami_hardware/power_measuring.h
    include/hanami_hardware/speed_measuring.h
    include/hanami_hardware/temperature_measuring.h
    include/hanami_hardware/value_container.h
    src/cpu_core.cpp
    src/cpu_package.cpp
    src/cpu_thread.cpp
    src/host.cpp
    src/power_measuring.cpp
    src/speed_measuring.cpp
    src/temperature_measuring.cpp
    src/value_container.cpp
)

target_include_directories(hanami_hardware PUBLIC
    include/
    src/
    ../hanami_common/include
    ../hanami_cpu/include
)

target_link_libraries(hanami_hardware PUBLIC
    hanami_common
    hanami_cpu
)

set_target_properties(hanami_hardware PROPERTIES POSITION_INDEPENDENT_CODE ON)
