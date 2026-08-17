cmake_minimum_required(VERSION 3.28)

project(
  fcitx5-windows-next
  VERSION 0.1.0
  DESCRIPTION "Native Fcitx5 platform frontend for Windows"
  LANGUAGES CXX)

include(CTest)

option(FCITX_WARNINGS_AS_ERRORS "Treat project warnings as errors" ON)
option(FCITX_ENABLE_MSVC_ANALYZE "Run MSVC static analysis" OFF)
option(FCITX_BUILD_BENCHMARKS "Build repeatable microbenchmarks" ON)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
include(CompilerOptions)

configure_file(
  cmake/version.h.in
  "${CMAKE_CURRENT_BINARY_DIR}/generated/fcitx5_windows/version.h"
  @ONLY)

add_library(fcitx5_windows_common STATIC src/common/version.cpp)
add_library(fcitx5::windows_common ALIAS fcitx5_windows_common)

target_compile_features(fcitx5_windows_common PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_windows_common
  PUBLIC "$<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/generated>")
fcitx_apply_project_options(fcitx5_windows_common)

add_library(fcitx5_protocol STATIC protocol/protocol.cpp)
add_library(fcitx5::protocol ALIAS fcitx5_protocol)
target_compile_features(fcitx5_protocol PUBLIC cxx_std_20)
target_include_directories(fcitx5_protocol PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/protocol")
fcitx_apply_project_options(fcitx5_protocol)

add_library(fcitx5_ipc_client STATIC src/ipc/pipe_client.cpp)
add_library(fcitx5::ipc_client ALIAS fcitx5_ipc_client)
target_compile_features(fcitx5_ipc_client PUBLIC cxx_std_20)
target_include_directories(fcitx5_ipc_client PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/ipc")
target_link_libraries(fcitx5_ipc_client PUBLIC fcitx5::protocol)
fcitx_apply_project_options(fcitx5_ipc_client)

add_executable(fcitx5_mock_engine src/engine/mock_engine_main.cpp)
set_target_properties(fcitx5_mock_engine PROPERTIES OUTPUT_NAME "fcitx5-mock-engine")
target_link_libraries(fcitx5_mock_engine PRIVATE fcitx5::protocol fcitx5::ipc_client)
fcitx_apply_project_options(fcitx5_mock_engine)
fcitx_apply_binary_hardening(fcitx5_mock_engine)

add_library(
  fcitx5_tsf SHARED
  src/tsf/module.cpp
  src/tsf/registration.cpp
  src/tsf/text_service.cpp
  src/tsf/fcitx5-tsf.def)
set_target_properties(fcitx5_tsf PROPERTIES PREFIX "" OUTPUT_NAME "fcitx5-tsf")
target_include_directories(fcitx5_tsf PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
target_link_libraries(fcitx5_tsf PRIVATE fcitx5::ipc_client uuid ole32 advapi32)
fcitx_apply_project_options(fcitx5_tsf)
fcitx_apply_binary_hardening(fcitx5_tsf)

if(BUILD_TESTING)
  add_executable(fcitx5_version_test tests/unit/version_test.cpp)
  target_link_libraries(fcitx5_version_test PRIVATE fcitx5::windows_common)
  fcitx_apply_project_options(fcitx5_version_test)
  fcitx_apply_binary_hardening(fcitx5_version_test)
  add_test(NAME version-contract COMMAND fcitx5_version_test)

  add_executable(fcitx5_protocol_test tests/unit/protocol_test.cpp)
  target_link_libraries(fcitx5_protocol_test PRIVATE fcitx5::protocol)
  fcitx_apply_project_options(fcitx5_protocol_test)
  fcitx_apply_binary_hardening(fcitx5_protocol_test)
  add_test(NAME protocol-contract COMMAND fcitx5_protocol_test)

  add_executable(fcitx5_ipc_roundtrip_test tests/integration/ipc_roundtrip_test.cpp)
  target_link_libraries(fcitx5_ipc_roundtrip_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_roundtrip_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_roundtrip_test)
  add_test(NAME ipc-key-commit-roundtrip
           COMMAND fcitx5_ipc_roundtrip_test $<TARGET_FILE:fcitx5_mock_engine>)

  add_executable(fcitx5_tsf_module_test tests/unit/tsf_module_test.cpp)
  target_include_directories(fcitx5_tsf_module_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
  target_link_libraries(fcitx5_tsf_module_test PRIVATE uuid ole32)
  fcitx_apply_project_options(fcitx5_tsf_module_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_module_test)
  add_test(NAME tsf-module-activation
           COMMAND fcitx5_tsf_module_test $<TARGET_FILE:fcitx5_tsf>)

  add_executable(fcitx5_tsf_key_commit_test tests/integration/tsf_key_commit_test.cpp)
  target_include_directories(fcitx5_tsf_key_commit_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
  target_link_libraries(fcitx5_tsf_key_commit_test PRIVATE uuid ole32)
  fcitx_apply_project_options(fcitx5_tsf_key_commit_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_key_commit_test)
  add_test(NAME tsf-key-commit-e2e
           COMMAND fcitx5_tsf_key_commit_test
                   $<TARGET_FILE:fcitx5_tsf>
                   $<TARGET_FILE:fcitx5_mock_engine>)
endif()

if(FCITX_BUILD_BENCHMARKS)
  add_executable(fcitx5_ipc_codec_bench tests/perf/ipc_codec_bench.cpp)
  target_link_libraries(fcitx5_ipc_codec_bench PRIVATE fcitx5::protocol)
  fcitx_apply_project_options(fcitx5_ipc_codec_bench)
  fcitx_apply_binary_hardening(fcitx5_ipc_codec_bench)

  add_executable(fcitx5_key_roundtrip_bench tests/perf/key_roundtrip_bench.cpp)
  target_link_libraries(fcitx5_key_roundtrip_bench PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_key_roundtrip_bench)
  fcitx_apply_binary_hardening(fcitx5_key_roundtrip_bench)
endif()
