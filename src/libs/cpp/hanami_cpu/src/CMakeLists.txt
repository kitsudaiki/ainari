qt_add_library(hanami_cpu
    ../include/hanami_cpu/cpu.h
    ../include/hanami_cpu/memory.h
    ../include/hanami_cpu/rapl.h
    cpu.cpp
    memory.cpp
    rapl.cpp
)
target_include_directories(hanami_cpu PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
    ../../../third-party-libs/json/include
    ../../hanami_common/include
    ../include
)

target_link_libraries(hanami_cpu PUBLIC
    # Remove: L../../hanami_common/src
    # Remove: L../../hanami_common/src/debug
    # Remove: L../../hanami_common/src/release
    # Remove: qt
    hanami_common
)

install(TARGETS hanami_cpu
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    FRAMEWORK DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
