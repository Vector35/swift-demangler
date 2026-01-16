cmake_minimum_required(VERSION 3.16)
project(SwiftDemangle VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

set(SWIFT_DEMANGLING_DIR ${CMAKE_CURRENT_SOURCE_DIR}/vendor)

if(NOT EXISTS ${SWIFT_DEMANGLING_DIR}/lib/Demangler.cpp)
    message(FATAL_ERROR
        "Swift demangling sources not found.\n"
        "Run: ./scripts/extract-swift-demangling.sh /path/to/swift")
endif()

file(GLOB SWIFT_DEMANGLING_SOURCES "${SWIFT_DEMANGLING_DIR}/lib/*.cpp")
list(FILTER SWIFT_DEMANGLING_SOURCES EXCLUDE REGEX ".*CrashReporter\\.cpp$")

add_library(swiftDemanglingInternal OBJECT ${SWIFT_DEMANGLING_SOURCES})
set_target_properties(swiftDemanglingInternal PROPERTIES CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN ON)
target_include_directories(swiftDemanglingInternal PRIVATE ${SWIFT_DEMANGLING_DIR}/include)
target_compile_definitions(swiftDemanglingInternal PRIVATE
    LLVM_DISABLE_ABI_BREAKING_CHECKS_ENFORCING=1
    SWIFT_SUPPORT_OLD_MANGLING=1
    SWIFT_STDLIB_HAS_TYPE_PRINTING=1
)
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(swiftDemanglingInternal PRIVATE
        -Wno-unused-parameter
        -Wno-covered-switch-default
    )
endif()

add_library(swift_demangle STATIC src/swift_demangle.cpp)
set_target_properties(swift_demangle PROPERTIES CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN ON)


target_include_directories(swift_demangle PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_include_directories(swift_demangle PRIVATE ${SWIFT_DEMANGLING_DIR}/include)
target_link_libraries(swift_demangle PRIVATE swiftDemanglingInternal)

option(BUILD_CLI "Build the command-line tool" ON)
if(BUILD_CLI)
    add_executable(swift-demangle src/main.cpp)
    target_link_libraries(swift-demangle PRIVATE swift_demangle)
endif()

install(TARGETS swift_demangle
    ARCHIVE DESTINATION lib
)
install(FILES include/swift_demangle.h DESTINATION include)

message(STATUS "")
message(STATUS "Swift Demangle Library Configuration:")
message(STATUS "  Vendor dir: ${SWIFT_DEMANGLING_DIR}")
if(EXISTS ${SWIFT_DEMANGLING_DIR}/VERSION)
    file(READ ${SWIFT_DEMANGLING_DIR}/VERSION SWIFT_VERSION)
    string(STRIP "${SWIFT_VERSION}" SWIFT_VERSION)
    message(STATUS "  Swift version: ${SWIFT_VERSION}")
endif()
message(STATUS "  Build CLI: ${BUILD_CLI}")
message(STATUS "")
