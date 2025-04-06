cmake_minimum_required(VERSION 3.16)
project(tests VERSION 0.1 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_INCLUDE_CURRENT_DIR ON)

add_subdirectory(unit_tests)
add_subdirectory(memory_leak_tests)
