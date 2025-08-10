cmake_minimum_required(VERSION 3.16)
project(Ainari VERSION 0.3.3 LANGUAGES C CXX)

set(CMAKE_INCLUDE_CURRENT_DIR ON)

# INFO (kitsudaiki): This is a workaround to be able to run valgrind for memory analytics.
#                    See github-issue: https://github.com/llvm/llvm-project/issues/56550
#set(CMAKE_C_FLAGS_DEBUG "${CMAKE_C_FLAGS_DEBUG} -gdwarf-4  -fsanitize=address -fno-omit-frame-pointer")
#set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -gdwarf-4  -fsanitize=address -fno-omit-frame-pointer")
set(CMAKE_C_FLAGS_DEBUG "${CMAKE_C_FLAGS_DEBUG} -gdwarf-4")
set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -gdwarf-4")

add_subdirectory(src/libs/cpp/ainari_core)
