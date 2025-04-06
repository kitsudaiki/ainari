cmake_minimum_required(VERSION 3.16)
project(hanami_cpu VERSION 0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(hanami_cpu STATIC
    include/hanami_cpu/cpu.h
    include/hanami_cpu/memory.h
    include/hanami_cpu/rapl.h
    src/cpu.cpp
    src/memory.cpp
    src/rapl.cpp
)

target_include_directories(hanami_cpu PUBLIC
    include/
    src/
    ../hanami_common/include
)

target_link_libraries(hanami_cpu PUBLIC
    hanami_common
)

set_target_properties(hanami_cpu PROPERTIES POSITION_INDEPENDENT_CODE ON)
