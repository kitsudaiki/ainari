qt_add_library(hanami_hardware
    ../include/hanami_hardware/cpu_core.h
    ../include/hanami_hardware/cpu_package.h
    ../include/hanami_hardware/cpu_thread.h
    ../include/hanami_hardware/host.h
    ../include/hanami_hardware/power_measuring.h
    ../include/hanami_hardware/speed_measuring.h
    ../include/hanami_hardware/temperature_measuring.h
    ../include/hanami_hardware/value_container.h
    cpu_core.cpp
    cpu_package.cpp
    cpu_thread.cpp
    host.cpp
    power_measuring.cpp
    speed_measuring.cpp
    temperature_measuring.cpp
    value_container.cpp
)
target_include_directories(hanami_hardware PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
    ../../../third-party-libs/json/include
    ../../hanami_common/include
    ../../hanami_cpu/include
    ../include
)

target_link_libraries(hanami_hardware PUBLIC
    # Remove: L../../hanami_common/src
    # Remove: L../../hanami_common/src/debug
    # Remove: L../../hanami_common/src/release
    # Remove: L../../hanami_cpu/src
    # Remove: L../../hanami_cpu/src/debug
    # Remove: L../../hanami_cpu/src/release
    # Remove: qt
    hanami_common
    hanami_cpu
)

install(TARGETS hanami_hardware
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    FRAMEWORK DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
