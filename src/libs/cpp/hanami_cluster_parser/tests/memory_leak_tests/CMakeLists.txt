add_executable(hanami_cluster_parser_memory_leak_tests
    cluster_parsestring_test.cpp
    cluster_parsestring_test.h
    main.cpp
)

target_include_directories(hanami_cluster_parser_memory_leak_tests PRIVATE
    ../../include/
    ../../src/
)

target_link_libraries(hanami_cluster_parser_memory_leak_tests PRIVATE
    hanami_cluster_parser
    hanami_common
)
