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
option(FCITX_BUILD_FUZZERS "Build security-boundary fuzz harnesses" ON)
option(FCITX_DEVELOPMENT_PEER_EXCEPTION
       "Allow explicitly marked same-user/session peers in development builds" OFF)

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

add_library(
  fcitx5_platform STATIC
  src/platform/runtime_identity.cpp
  src/platform/pipe_security.cpp)
add_library(fcitx5::platform ALIAS fcitx5_platform)
target_compile_features(fcitx5_platform PUBLIC cxx_std_20)
target_include_directories(fcitx5_platform PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/platform")
target_link_libraries(fcitx5_platform PUBLIC advapi32 user32)
fcitx_apply_project_options(fcitx5_platform)

add_library(
  fcitx5_launcher_state STATIC
  src/launcher/state_machine.cpp
  src/launcher/state_store.cpp)
add_library(fcitx5::launcher_state ALIAS fcitx5_launcher_state)
target_compile_features(fcitx5_launcher_state PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_launcher_state PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/launcher")
target_link_libraries(fcitx5_launcher_state PUBLIC shell32 ole32)
fcitx_apply_project_options(fcitx5_launcher_state)

add_library(
  fcitx5_ipc_client STATIC
  src/ipc/launcher_client.cpp
  src/ipc/pipe_client.cpp
  src/ipc/peer_verification.cpp)
add_library(fcitx5::ipc_client ALIAS fcitx5_ipc_client)
target_compile_features(fcitx5_ipc_client PUBLIC cxx_std_20)
target_include_directories(fcitx5_ipc_client PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/ipc")
target_link_libraries(fcitx5_ipc_client PUBLIC fcitx5::protocol fcitx5::platform)
if(FCITX_DEVELOPMENT_PEER_EXCEPTION)
  target_compile_definitions(fcitx5_ipc_client PRIVATE FCITX_DEVELOPMENT_PEER_EXCEPTION=1)
endif()
fcitx_apply_project_options(fcitx5_ipc_client)

add_executable(fcitx5_mock_engine src/engine/mock_engine_main.cpp)
set_target_properties(fcitx5_mock_engine PROPERTIES OUTPUT_NAME "fcitx5-mock-engine")
target_link_libraries(
  fcitx5_mock_engine
  PRIVATE fcitx5::windows_common fcitx5::protocol fcitx5::ipc_client fcitx5::platform)
fcitx_apply_project_options(fcitx5_mock_engine)
fcitx_apply_binary_hardening(fcitx5_mock_engine)

add_executable(fcitx5_launcher src/launcher/launcher_main.cpp)
set_target_properties(fcitx5_launcher PROPERTIES OUTPUT_NAME "fcitx5-launcher")
target_link_libraries(
  fcitx5_launcher
  PRIVATE fcitx5::windows_common fcitx5::launcher_state fcitx5::protocol
          fcitx5::ipc_client fcitx5::platform)
fcitx_apply_project_options(fcitx5_launcher)
fcitx_apply_binary_hardening(fcitx5_launcher)

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
  add_test(NAME launcher-version COMMAND fcitx5_launcher --version)
  add_test(NAME mock-engine-version COMMAND fcitx5_mock_engine --version)

  add_executable(fcitx5_protocol_test tests/unit/protocol_test.cpp)
  target_link_libraries(fcitx5_protocol_test PRIVATE fcitx5::protocol)
  fcitx_apply_project_options(fcitx5_protocol_test)
  fcitx_apply_binary_hardening(fcitx5_protocol_test)
  add_test(NAME protocol-contract COMMAND fcitx5_protocol_test)

  add_executable(fcitx5_runtime_identity_test tests/unit/runtime_identity_test.cpp)
  target_link_libraries(fcitx5_runtime_identity_test PRIVATE fcitx5::platform)
  fcitx_apply_project_options(fcitx5_runtime_identity_test)
  fcitx_apply_binary_hardening(fcitx5_runtime_identity_test)
  add_test(NAME runtime-identity-contract COMMAND fcitx5_runtime_identity_test)

  add_executable(fcitx5_launcher_state_test tests/unit/launcher_state_test.cpp)
  target_link_libraries(fcitx5_launcher_state_test PRIVATE fcitx5::launcher_state)
  fcitx_apply_project_options(fcitx5_launcher_state_test)
  fcitx_apply_binary_hardening(fcitx5_launcher_state_test)
  add_test(NAME launcher-state-model COMMAND fcitx5_launcher_state_test)

  add_executable(fcitx5_ipc_roundtrip_test tests/integration/ipc_roundtrip_test.cpp)
  target_link_libraries(fcitx5_ipc_roundtrip_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_roundtrip_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_roundtrip_test)
  add_test(NAME ipc-key-commit-roundtrip
           COMMAND fcitx5_ipc_roundtrip_test $<TARGET_FILE:fcitx5_mock_engine>)

  add_executable(fcitx5_ipc_multi_client_test tests/integration/ipc_multi_client_test.cpp)
  target_link_libraries(fcitx5_ipc_multi_client_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_multi_client_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_multi_client_test)
  add_test(NAME ipc-multi-client
           COMMAND fcitx5_ipc_multi_client_test $<TARGET_FILE:fcitx5_mock_engine>)

  add_executable(fcitx5_ipc_late_response_test
                 tests/integration/ipc_late_response_test.cpp)
  target_link_libraries(fcitx5_ipc_late_response_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_late_response_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_late_response_test)
  add_test(NAME ipc-late-response-reconnect
           COMMAND fcitx5_ipc_late_response_test $<TARGET_FILE:fcitx5_mock_engine>)

  add_executable(fcitx5_launcher_integration_test tests/integration/launcher_integration_test.cpp)
  target_link_libraries(
    fcitx5_launcher_integration_test PRIVATE fcitx5::ipc_client fcitx5::launcher_state)
  fcitx_apply_project_options(fcitx5_launcher_integration_test)
  fcitx_apply_binary_hardening(fcitx5_launcher_integration_test)
  add_test(NAME launcher-engine-lifecycle
           COMMAND fcitx5_launcher_integration_test
                   $<TARGET_FILE:fcitx5_launcher>
                   $<TARGET_FILE:fcitx5_mock_engine>)

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

if(FCITX_BUILD_FUZZERS)
  add_executable(fcitx5_protocol_fuzz tests/fuzz/protocol_fuzz.cpp)
  target_link_libraries(fcitx5_protocol_fuzz PRIVATE fcitx5::protocol)
  fcitx_apply_project_options(fcitx5_protocol_fuzz)
  fcitx_apply_binary_hardening(fcitx5_protocol_fuzz)
  if(BUILD_TESTING)
    add_test(NAME protocol-fuzz-smoke COMMAND fcitx5_protocol_fuzz)
  endif()
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
