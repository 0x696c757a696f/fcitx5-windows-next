cmake_minimum_required(VERSION 3.28)

project(
  fcitx5-windows-next
  VERSION 0.1.0
  DESCRIPTION "Native Fcitx5 platform frontend for Windows"
  LANGUAGES C CXX)

include(CTest)

set(FCITX_WTL_INCLUDE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/wtl-10.1.0/lib/native/include"
    CACHE PATH "Path to the pinned WTL 10.1.0 headers")
if(NOT EXISTS "${FCITX_WTL_INCLUDE_DIR}/atlapp.h")
  message(FATAL_ERROR
          "Pinned WTL headers are missing. Run tools/prepare-wtl.ps1.")
endif()

set(FCITX_NLOHMANN_JSON_INCLUDE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/nlohmann-json-3.12.0"
    CACHE PATH "Path to the pinned nlohmann/json 3.12.0 header")
if(NOT EXISTS "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}/nlohmann/json.hpp")
  message(FATAL_ERROR
          "Pinned nlohmann/json header is missing. Run tools/prepare-package-dependencies.ps1.")
endif()

set(FCITX_MINIZ_SOURCE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/out/toolchains/miniz-3.1.2"
    CACHE PATH "Path to the pinned miniz 3.1.2 sources")
if(NOT EXISTS "${FCITX_MINIZ_SOURCE_DIR}/miniz.c" OR
   NOT EXISTS "${FCITX_MINIZ_SOURCE_DIR}/miniz.h")
  message(FATAL_ERROR
          "Pinned miniz sources are missing. Run tools/prepare-package-dependencies.ps1.")
endif()

option(FCITX_WARNINGS_AS_ERRORS "Treat project warnings as errors" ON)
option(FCITX_ENABLE_MSVC_ANALYZE "Run MSVC static analysis" OFF)
option(FCITX_BUILD_BENCHMARKS "Build repeatable microbenchmarks" ON)
option(FCITX_BUILD_FUZZERS "Build security-boundary fuzz harnesses" ON)
option(FCITX_DEVELOPMENT_PEER_EXCEPTION
       "Allow explicitly marked same-user/session peers in development builds" OFF)

set(FCITX_RELEASE_CHANNEL "stable" CACHE STRING "Release identity: stable, beta, or nightly")
set_property(CACHE FCITX_RELEASE_CHANNEL PROPERTY STRINGS stable beta nightly)
if(FCITX_RELEASE_CHANNEL STREQUAL "stable")
  set(FCITX_RELEASE_CHANNEL_INDEX 0)
elseif(FCITX_RELEASE_CHANNEL STREQUAL "beta")
  set(FCITX_RELEASE_CHANNEL_INDEX 1)
elseif(FCITX_RELEASE_CHANNEL STREQUAL "nightly")
  set(FCITX_RELEASE_CHANNEL_INDEX 2)
else()
  message(FATAL_ERROR "FCITX_RELEASE_CHANNEL must be stable, beta, or nightly")
endif()

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
include(CompilerOptions)

configure_file(
  cmake/version.h.in
  "${CMAKE_CURRENT_BINARY_DIR}/generated/fcitx5_windows/version.h"
  @ONLY)
configure_file(
  cmake/release_identity.h.in
  "${CMAKE_CURRENT_BINARY_DIR}/generated/fcitx5_windows/release_identity.h"
  @ONLY)

add_library(fcitx5_release_identity INTERFACE)
add_library(fcitx5::release_identity ALIAS fcitx5_release_identity)
target_include_directories(
  fcitx5_release_identity
  INTERFACE "$<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/generated>")

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

add_library(fcitx5_candidate_model STATIC src/candidate/candidate_model.cpp)
add_library(fcitx5::candidate_model ALIAS fcitx5_candidate_model)
target_compile_features(fcitx5_candidate_model PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_candidate_model PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/candidate")
fcitx_apply_project_options(fcitx5_candidate_model)

add_library(fcitx5_miniz STATIC "${FCITX_MINIZ_SOURCE_DIR}/miniz.c")
target_include_directories(fcitx5_miniz SYSTEM PUBLIC "${FCITX_MINIZ_SOURCE_DIR}")
target_compile_definitions(fcitx5_miniz PRIVATE MINIZ_NO_ZLIB_APIS)
if(MSVC)
  target_compile_options(fcitx5_miniz PRIVATE /W3 /sdl)
endif()

add_library(
  fcitx5_package_core STATIC
  src/package/package_core.cpp
  src/package/package_archive.cpp
  src/package/repository.cpp)
add_library(fcitx5::package_core ALIAS fcitx5_package_core)
target_compile_features(fcitx5_package_core PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_package_core PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/package")
target_include_directories(
  fcitx5_package_core SYSTEM PRIVATE "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}")
target_link_libraries(fcitx5_package_core PUBLIC bcrypt crypt32 PRIVATE fcitx5_miniz)
fcitx_apply_project_options(fcitx5_package_core)
if(MSVC)
  target_compile_options(fcitx5_package_core PRIVATE /external:W0 /analyze:external-)
endif()

add_library(fcitx5_provider_policy STATIC src/package/provider_policy.cpp)
add_library(fcitx5::provider_policy ALIAS fcitx5_provider_policy)
target_include_directories(
  fcitx5_provider_policy PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/package")
target_link_libraries(fcitx5_provider_policy PUBLIC fcitx5::package_core)
fcitx_apply_project_options(fcitx5_provider_policy)

add_library(fcitx5_deployment_core STATIC src/updater/deployment_core.cpp)
add_library(fcitx5::deployment_core ALIAS fcitx5_deployment_core)
target_compile_features(fcitx5_deployment_core PUBLIC cxx_std_20)
target_include_directories(fcitx5_deployment_core PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/updater")
target_include_directories(fcitx5_deployment_core SYSTEM PRIVATE "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}")
fcitx_apply_project_options(fcitx5_deployment_core)
if(MSVC)
  target_compile_options(fcitx5_deployment_core PRIVATE /external:W0 /analyze:external-)
endif()

add_library(fcitx5_config STATIC src/config/config_parser.cpp)
add_library(fcitx5::config ALIAS fcitx5_config)
target_compile_features(fcitx5_config PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_config
  PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/config")
target_include_directories(
  fcitx5_config SYSTEM
  PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/third_party/tomlplusplus/include")
target_compile_definitions(fcitx5_config PRIVATE TOML_EXCEPTIONS=1)
fcitx_apply_project_options(fcitx5_config)
if(MSVC)
  target_compile_options(fcitx5_config PRIVATE /external:W0 /analyze:external-)
endif()

add_library(fcitx5_process_execution STATIC src/config/process_execution.cpp)
add_library(fcitx5::process_execution ALIAS fcitx5_process_execution)
target_compile_features(fcitx5_process_execution PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_process_execution PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/config")
fcitx_apply_project_options(fcitx5_process_execution)

add_library(
  fcitx5_platform STATIC
  src/platform/runtime_identity.cpp
  src/platform/pipe_security.cpp)
add_library(fcitx5::platform ALIAS fcitx5_platform)
target_compile_features(fcitx5_platform PUBLIC cxx_std_20)
target_include_directories(fcitx5_platform PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/platform")
target_link_libraries(fcitx5_platform PUBLIC fcitx5::release_identity advapi32 user32)
fcitx_apply_project_options(fcitx5_platform)

add_library(
  fcitx5_launcher_state STATIC
  src/launcher/state_machine.cpp
  src/launcher/state_store.cpp)
add_library(fcitx5::launcher_state ALIAS fcitx5_launcher_state)
target_compile_features(fcitx5_launcher_state PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_launcher_state PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/launcher")
target_link_libraries(
  fcitx5_launcher_state PUBLIC fcitx5::release_identity shell32 ole32)
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

add_library(fcitx5_tsf_activation_guard STATIC src/tsf/activation_guard.cpp)
add_library(fcitx5::tsf_activation_guard ALIAS fcitx5_tsf_activation_guard)
target_compile_features(fcitx5_tsf_activation_guard PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_tsf_activation_guard PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
target_link_libraries(fcitx5_tsf_activation_guard PUBLIC fcitx5::release_identity shell32)
fcitx_apply_project_options(fcitx5_tsf_activation_guard)

add_executable(fcitx5_mock_engine src/engine/mock_engine_main.cpp)
set_target_properties(fcitx5_mock_engine PROPERTIES OUTPUT_NAME "fcitx5-mock-engine")
target_link_libraries(
  fcitx5_mock_engine
  PRIVATE fcitx5::windows_common fcitx5::protocol fcitx5::ipc_client fcitx5::platform)
fcitx_apply_project_options(fcitx5_mock_engine)
fcitx_apply_binary_hardening(fcitx5_mock_engine)

add_executable(
  fcitx5_launcher WIN32
  src/launcher/launcher_main.cpp
  src/launcher/tray_icon.cpp
  resources/windows/launcher.rc)
set_target_properties(fcitx5_launcher PROPERTIES OUTPUT_NAME "fcitx5-launcher")
target_link_libraries(
  fcitx5_launcher
  PRIVATE fcitx5::windows_common fcitx5::launcher_state fcitx5::protocol
          fcitx5::ipc_client fcitx5::platform)
target_include_directories(
  fcitx5_launcher PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/resources/windows")
fcitx_apply_project_options(fcitx5_launcher)
fcitx_apply_binary_hardening(fcitx5_launcher)

add_executable(fcitx5_control src/control/control_main.cpp)
set_target_properties(fcitx5_control PROPERTIES OUTPUT_NAME "fcitx5-control")
target_link_libraries(
  fcitx5_control
  PRIVATE fcitx5::windows_common fcitx5::config fcitx5::protocol
          fcitx5::ipc_client fcitx5::platform fcitx5::package_core
          fcitx5::deployment_core fcitx5::tsf_activation_guard
          fcitx5::process_execution shell32 ole32)
fcitx_apply_project_options(fcitx5_control)
fcitx_apply_binary_hardening(fcitx5_control)

add_executable(
  fcitx5_config_app WIN32
  src/config/app_main.cpp
  resources/windows/app.rc)
set_target_properties(fcitx5_config_app PROPERTIES OUTPUT_NAME "fcitx5-config")
target_link_libraries(
  fcitx5_config_app
  PRIVATE fcitx5::windows_common fcitx5::release_identity fcitx5::process_execution d2d1
          dwrite shell32 user32 comctl32)
target_include_directories(fcitx5_config_app SYSTEM PRIVATE "${FCITX_WTL_INCLUDE_DIR}")
target_include_directories(fcitx5_config_app SYSTEM PRIVATE "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}")
if(MSVC)
  target_compile_options(fcitx5_config_app PRIVATE /external:W0 /analyze:external-)
endif()
fcitx_apply_project_options(fcitx5_config_app)
fcitx_apply_binary_hardening(fcitx5_config_app)
add_custom_command(
  TARGET fcitx5_config_app POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_directory
          "${CMAKE_CURRENT_SOURCE_DIR}/locales"
          "$<TARGET_FILE_DIR:fcitx5_config_app>/locales"
  VERBATIM)

add_library(fcitx5_candidate_layout STATIC src/ui/candidate_layout.cpp)
add_library(fcitx5::candidate_layout ALIAS fcitx5_candidate_layout)
target_compile_features(fcitx5_candidate_layout PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_candidate_layout PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/ui")
fcitx_apply_project_options(fcitx5_candidate_layout)

add_library(fcitx5_candidate_interaction STATIC src/ui/candidate_interaction.cpp)
add_library(fcitx5::candidate_interaction ALIAS fcitx5_candidate_interaction)
target_compile_features(fcitx5_candidate_interaction PUBLIC cxx_std_20)
target_include_directories(
  fcitx5_candidate_interaction PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src/ui")
target_link_libraries(fcitx5_candidate_interaction PUBLIC d2d1 user32)
fcitx_apply_project_options(fcitx5_candidate_interaction)

add_executable(
  fcitx5_ui WIN32
  src/ui/ui_main.cpp
  resources/windows/app.rc)
set_target_properties(fcitx5_ui PROPERTIES OUTPUT_NAME "fcitx5-ui")
  target_link_libraries(
  fcitx5_ui
  PRIVATE fcitx5::candidate_model fcitx5::candidate_layout fcitx5::candidate_interaction fcitx5::config
          fcitx5::protocol fcitx5::ipc_client fcitx5::platform
          d2d1 dwrite shell32 ole32)
fcitx_apply_project_options(fcitx5_ui)
fcitx_apply_binary_hardening(fcitx5_ui)
add_custom_command(
  TARGET fcitx5_ui POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_directory
          "${CMAKE_CURRENT_SOURCE_DIR}/resources"
          "$<TARGET_FILE_DIR:fcitx5_ui>/resources"
  VERBATIM)

add_library(
  fcitx5_tsf SHARED
  src/tsf/module.cpp
  src/tsf/candidate_ui_element.cpp
  src/tsf/input_scope_guids.cpp
  src/tsf/input_profiles.cpp
  src/tsf/registration.cpp
  src/tsf/text_service.cpp
  src/tsf/fcitx5-tsf.def
  resources/windows/app.rc)
set_target_properties(fcitx5_tsf PROPERTIES PREFIX "" OUTPUT_NAME "fcitx5-tsf")
target_include_directories(fcitx5_tsf PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
target_link_libraries(
  fcitx5_tsf
  PRIVATE fcitx5::release_identity fcitx5::ipc_client fcitx5::tsf_activation_guard uuid ole32
          oleaut32 advapi32)
if(FCITX_DEVELOPMENT_PEER_EXCEPTION)
  target_compile_definitions(fcitx5_tsf PRIVATE FCITX_DEVELOPMENT_PEER_EXCEPTION=1)
endif()
fcitx_apply_project_options(fcitx5_tsf)
fcitx_apply_binary_hardening(fcitx5_tsf)

add_executable(fcitx5_register src/register/register_main.cpp)
set_target_properties(fcitx5_register PROPERTIES OUTPUT_NAME "fcitx5-register")
target_include_directories(fcitx5_register PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
target_link_libraries(fcitx5_register PRIVATE fcitx5::windows_common advapi32 ole32)
fcitx_apply_project_options(fcitx5_register)
fcitx_apply_binary_hardening(fcitx5_register)

add_executable(
  fcitx5_bootstrap WIN32
  src/bootstrap/bootstrap_main.cpp
  resources/windows/app.rc)
set_target_properties(fcitx5_bootstrap PROPERTIES OUTPUT_NAME "fcitx5-bootstrap")
target_link_libraries(fcitx5_bootstrap PRIVATE fcitx5::windows_common shell32 user32)
fcitx_apply_project_options(fcitx5_bootstrap)
fcitx_apply_binary_hardening(fcitx5_bootstrap)

add_executable(fcitx5_package src/package/package_main.cpp)
set_target_properties(fcitx5_package PROPERTIES OUTPUT_NAME "fcitx5-package")
target_link_libraries(fcitx5_package PRIVATE fcitx5::windows_common fcitx5::package_core)
fcitx_apply_project_options(fcitx5_package)
fcitx_apply_binary_hardening(fcitx5_package)

add_executable(fcitx5_downloader src/package/downloader_main.cpp)
set_target_properties(fcitx5_downloader PROPERTIES OUTPUT_NAME "fcitx5-downloader")
target_link_libraries(
  fcitx5_downloader PRIVATE fcitx5::windows_common fcitx5::package_core winhttp advapi32)
fcitx_apply_project_options(fcitx5_downloader)
fcitx_apply_binary_hardening(fcitx5_downloader)

add_executable(fcitx5_deployer src/package/deployer_main.cpp)
set_target_properties(fcitx5_deployer PROPERTIES OUTPUT_NAME "fcitx5-deployer")
target_link_libraries(
  fcitx5_deployer PRIVATE fcitx5::windows_common fcitx5::package_core advapi32 shell32)
fcitx_apply_project_options(fcitx5_deployer)
fcitx_apply_binary_hardening(fcitx5_deployer)

add_executable(fcitx5_provider src/package/provider_main.cpp)
set_target_properties(fcitx5_provider PROPERTIES OUTPUT_NAME "fcitx5-provider")
target_link_libraries(
  fcitx5_provider PRIVATE fcitx5::windows_common fcitx5::provider_policy advapi32)
fcitx_apply_project_options(fcitx5_provider)
fcitx_apply_binary_hardening(fcitx5_provider)

add_executable(fcitx5_updater src/updater/updater_main.cpp)
set_target_properties(fcitx5_updater PROPERTIES OUTPUT_NAME "fcitx5-updater")
target_link_libraries(fcitx5_updater PRIVATE fcitx5::windows_common fcitx5::package_core
                                             fcitx5::deployment_core)
fcitx_apply_project_options(fcitx5_updater)
fcitx_apply_binary_hardening(fcitx5_updater)

if(BUILD_TESTING)
  add_executable(fcitx5_version_test tests/unit/version_test.cpp)
  target_link_libraries(fcitx5_version_test PRIVATE fcitx5::windows_common)
  fcitx_apply_project_options(fcitx5_version_test)
  fcitx_apply_binary_hardening(fcitx5_version_test)
  add_test(NAME version-contract COMMAND fcitx5_version_test)

  add_executable(fcitx5_release_identity_test tests/unit/release_identity_test.cpp)
  target_link_libraries(
    fcitx5_release_identity_test PRIVATE fcitx5::windows_common fcitx5::release_identity ole32)
  fcitx_apply_project_options(fcitx5_release_identity_test)
  fcitx_apply_binary_hardening(fcitx5_release_identity_test)
  add_test(NAME release-identity-contract COMMAND fcitx5_release_identity_test)

  add_executable(fcitx5_package_core_test tests/unit/package_core_test.cpp)
  target_include_directories(
    fcitx5_package_core_test SYSTEM PRIVATE "${FCITX_MINIZ_SOURCE_DIR}")
  target_link_libraries(
    fcitx5_package_core_test PRIVATE fcitx5::package_core fcitx5_miniz bcrypt)
  fcitx_apply_project_options(fcitx5_package_core_test)
  fcitx_apply_binary_hardening(fcitx5_package_core_test)
  add_test(NAME package-core-contract COMMAND fcitx5_package_core_test)

  add_executable(
    fcitx5_control_package_integration_test
    tests/integration/control_package_integration_test.cpp)
  target_include_directories(
    fcitx5_control_package_integration_test SYSTEM PRIVATE "${FCITX_MINIZ_SOURCE_DIR}")
  target_link_libraries(
    fcitx5_control_package_integration_test
    PRIVATE fcitx5::package_core fcitx5_miniz bcrypt crypt32)
  add_dependencies(
    fcitx5_control_package_integration_test fcitx5_control fcitx5_downloader)
  fcitx_apply_project_options(fcitx5_control_package_integration_test)
  fcitx_apply_binary_hardening(fcitx5_control_package_integration_test)
  add_test(
    NAME control-package-stopped-service-contract
    COMMAND fcitx5_control_package_integration_test
            "$<TARGET_FILE:fcitx5_control>" "$<TARGET_FILE:fcitx5_downloader>")
  set_tests_properties(control-package-stopped-service-contract PROPERTIES TIMEOUT 30)
  add_test(NAME launcher-version COMMAND fcitx5_launcher --version)
  add_test(NAME mock-engine-version COMMAND fcitx5_mock_engine --version)
  add_test(NAME control-version COMMAND fcitx5_control --version)
  add_test(NAME control-schema COMMAND fcitx5_control --schema)
  add_test(NAME control-startup-query COMMAND fcitx5_control --get-startup)
  add_test(NAME register-version COMMAND fcitx5_register --version)
  add_test(NAME package-version COMMAND fcitx5_package --version)
  add_executable(fcitx5_deployment_core_test tests/unit/deployment_core_test.cpp)
  target_link_libraries(fcitx5_deployment_core_test PRIVATE fcitx5::deployment_core)
  fcitx_apply_project_options(fcitx5_deployment_core_test)
  fcitx_apply_binary_hardening(fcitx5_deployment_core_test)
  add_test(NAME deployment-previous-known-good COMMAND fcitx5_deployment_core_test)
  add_test(NAME updater-version COMMAND fcitx5_updater --version)
  add_test(NAME downloader-version COMMAND fcitx5_downloader --version)
  add_test(NAME downloader-boundary-smoke COMMAND fcitx5_downloader --self-test)
  add_test(NAME deployer-version COMMAND fcitx5_deployer --version)
  add_test(NAME deployer-boundary-smoke COMMAND fcitx5_deployer --self-test)
  add_test(NAME provider-version COMMAND fcitx5_provider --version)

  add_executable(fcitx5_updater_cleanup_test tests/unit/updater_cleanup_test.cpp)
  target_link_libraries(fcitx5_updater_cleanup_test PRIVATE fcitx5::deployment_core)
  add_dependencies(fcitx5_updater_cleanup_test fcitx5_updater)
  fcitx_apply_project_options(fcitx5_updater_cleanup_test)
  fcitx_apply_binary_hardening(fcitx5_updater_cleanup_test)
  add_test(NAME updater-cleanup-previous
           COMMAND fcitx5_updater_cleanup_test "$<TARGET_FILE:fcitx5_updater>")

  add_executable(fcitx5_control_repository_rollback_test
                 tests/unit/control_repository_rollback_test.cpp)
  target_include_directories(
    fcitx5_control_repository_rollback_test SYSTEM PRIVATE
    "${FCITX_NLOHMANN_JSON_INCLUDE_DIR}")
  target_link_libraries(
    fcitx5_control_repository_rollback_test PRIVATE fcitx5::package_core bcrypt crypt32)
  add_dependencies(fcitx5_control_repository_rollback_test fcitx5_control)
  fcitx_apply_project_options(fcitx5_control_repository_rollback_test)
  fcitx_apply_binary_hardening(fcitx5_control_repository_rollback_test)
  add_test(NAME control-repository-rollback
           COMMAND fcitx5_control_repository_rollback_test "$<TARGET_FILE:fcitx5_control>")

  add_executable(fcitx5_provider_policy_test tests/unit/provider_policy_test.cpp)
  target_link_libraries(fcitx5_provider_policy_test PRIVATE fcitx5::provider_policy)
  fcitx_apply_project_options(fcitx5_provider_policy_test)
  fcitx_apply_binary_hardening(fcitx5_provider_policy_test)
  add_test(NAME provider-policy-contract COMMAND fcitx5_provider_policy_test)
  add_test(NAME control-config-roundtrip
           COMMAND fcitx5_control --data-root
                   "${CMAKE_CURRENT_BINARY_DIR}/control-test-data"
                   --apply-config "${CMAKE_CURRENT_SOURCE_DIR}/resources/config.toml")
  add_test(NAME config-ui-headless-smoke COMMAND fcitx5_config_app --self-test)
  add_test(NAME config-ui-i18n-check COMMAND fcitx5_config_app --check-i18n)
  add_test(NAME config-ui-resource-check COMMAND fcitx5_config_app --check-resources)
  add_test(NAME config-ui-behavior-contract COMMAND fcitx5_config_app --ui-contract-test)
  add_test(NAME config-ui-interaction-coverage
           COMMAND fcitx5_config_app --ui-interaction-test)

  add_executable(fcitx5_protocol_test tests/unit/protocol_test.cpp)
  target_link_libraries(fcitx5_protocol_test PRIVATE fcitx5::protocol)
  fcitx_apply_project_options(fcitx5_protocol_test)
  fcitx_apply_binary_hardening(fcitx5_protocol_test)
  add_test(NAME protocol-contract COMMAND fcitx5_protocol_test)

  add_executable(fcitx5_candidate_model_test tests/unit/candidate_model_test.cpp)
  target_link_libraries(fcitx5_candidate_model_test PRIVATE fcitx5::candidate_model)
  fcitx_apply_project_options(fcitx5_candidate_model_test)
  fcitx_apply_binary_hardening(fcitx5_candidate_model_test)
  add_test(NAME candidate-model-contract COMMAND fcitx5_candidate_model_test)

  add_executable(fcitx5_config_parser_test tests/unit/config_parser_test.cpp)
  target_link_libraries(fcitx5_config_parser_test PRIVATE fcitx5::config)
  fcitx_apply_project_options(fcitx5_config_parser_test)
  fcitx_apply_binary_hardening(fcitx5_config_parser_test)
  add_test(NAME config-toml-contract COMMAND fcitx5_config_parser_test)
  set_tests_properties(config-toml-contract PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")

  add_executable(fcitx5_process_execution_test tests/unit/process_execution_test.cpp)
  target_link_libraries(fcitx5_process_execution_test PRIVATE fcitx5::process_execution)
  fcitx_apply_project_options(fcitx5_process_execution_test)
  fcitx_apply_binary_hardening(fcitx5_process_execution_test)
  add_test(NAME process-execution-pipe COMMAND fcitx5_process_execution_test)

  add_executable(fcitx5_source_contract_test tests/unit/source_contract_test.cpp)
  fcitx_apply_project_options(fcitx5_source_contract_test)
  fcitx_apply_binary_hardening(fcitx5_source_contract_test)
  add_test(NAME source-contract COMMAND fcitx5_source_contract_test "${CMAKE_CURRENT_SOURCE_DIR}")

  add_executable(fcitx5_candidate_layout_test tests/unit/candidate_layout_test.cpp)
  target_link_libraries(fcitx5_candidate_layout_test PRIVATE fcitx5::candidate_layout)
  fcitx_apply_project_options(fcitx5_candidate_layout_test)
  fcitx_apply_binary_hardening(fcitx5_candidate_layout_test)
  add_test(NAME candidate-layout-contract COMMAND fcitx5_candidate_layout_test)

  add_executable(fcitx5_candidate_navigation_test
                 tests/unit/candidate_navigation_test.cpp)
  target_include_directories(
    fcitx5_candidate_navigation_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/engine")
  fcitx_apply_project_options(fcitx5_candidate_navigation_test)
  fcitx_apply_binary_hardening(fcitx5_candidate_navigation_test)
  add_test(NAME candidate-navigation-contract COMMAND fcitx5_candidate_navigation_test)

  add_executable(fcitx5_candidate_interaction_test tests/unit/candidate_interaction_test.cpp)
  target_link_libraries(fcitx5_candidate_interaction_test PRIVATE fcitx5::candidate_interaction)
  fcitx_apply_project_options(fcitx5_candidate_interaction_test)
  fcitx_apply_binary_hardening(fcitx5_candidate_interaction_test)
  add_test(NAME candidate-interaction-contract COMMAND fcitx5_candidate_interaction_test)
  add_test(NAME candidate-ui-device-smoke
           COMMAND fcitx5_ui --self-test --simulate-device-loss)
  add_test(NAME candidate-ui-safe-mode-smoke COMMAND fcitx5_ui --self-test --safe-mode)
  add_test(NAME candidate-ui-interaction-contract
           COMMAND fcitx5_ui --interaction-self-test)
  set_tests_properties(candidate-ui-interaction-contract PROPERTIES TIMEOUT 5)
  add_test(NAME candidate-ui-scroll-expansion-contract
           COMMAND fcitx5_ui --scroll-expansion-self-test)
  set_tests_properties(candidate-ui-scroll-expansion-contract PROPERTIES TIMEOUT 5)
  add_test(NAME candidate-ui-live-config-reflow
           COMMAND fcitx5_ui --self-test --reload-test)

  add_executable(
    fcitx5_candidate_ui_config_integration_test
    tests/integration/candidate_ui_config_integration_test.cpp)
  add_dependencies(
    fcitx5_candidate_ui_config_integration_test fcitx5_ui fcitx5_control)
  target_link_libraries(
    fcitx5_candidate_ui_config_integration_test PRIVATE fcitx5::config user32)
  fcitx_apply_project_options(fcitx5_candidate_ui_config_integration_test)
  fcitx_apply_binary_hardening(fcitx5_candidate_ui_config_integration_test)
  add_test(
    NAME candidate-ui-live-presentation-contract
    COMMAND fcitx5_candidate_ui_config_integration_test
            "$<TARGET_FILE:fcitx5_ui>" "$<TARGET_FILE:fcitx5_control>"
            "$<TARGET_FILE_DIR:fcitx5_ui>/resources")
  set_tests_properties(candidate-ui-live-presentation-contract PROPERTIES TIMEOUT 30)

  add_executable(
    fcitx5_candidate_ui_element_test
    tests/unit/candidate_ui_element_test.cpp
    src/tsf/candidate_ui_element.cpp)
  target_include_directories(
    fcitx5_candidate_ui_element_test
    PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf"
            "${CMAKE_CURRENT_SOURCE_DIR}/src/ipc")
  target_link_libraries(
    fcitx5_candidate_ui_element_test PRIVATE fcitx5::ipc_client ole32 oleaut32)
  fcitx_apply_project_options(fcitx5_candidate_ui_element_test)
  fcitx_apply_binary_hardening(fcitx5_candidate_ui_element_test)
  add_test(NAME tsf-uiless-candidate-contract COMMAND fcitx5_candidate_ui_element_test)

  add_executable(fcitx5_tsf_activation_guard_test tests/unit/tsf_activation_guard_test.cpp)
  target_link_libraries(
    fcitx5_tsf_activation_guard_test PRIVATE fcitx5::tsf_activation_guard)
  fcitx_apply_project_options(fcitx5_tsf_activation_guard_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_activation_guard_test)
  add_test(NAME tsf-activation-guard-contract COMMAND fcitx5_tsf_activation_guard_test)

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
  add_test(NAME ipc-multi-client-32
           COMMAND fcitx5_ipc_multi_client_test $<TARGET_FILE:fcitx5_mock_engine> 32)
  set_tests_properties(ipc-multi-client-32 PROPERTIES TIMEOUT 30)

  add_executable(fcitx5_ipc_idle_client_test tests/integration/ipc_idle_client_test.cpp)
  target_link_libraries(fcitx5_ipc_idle_client_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_idle_client_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_idle_client_test)
  add_test(NAME ipc-idle-client
           COMMAND fcitx5_ipc_idle_client_test $<TARGET_FILE:fcitx5_mock_engine>)
  set_tests_properties(ipc-idle-client PROPERTIES TIMEOUT 30)

  add_executable(fcitx5_ipc_generation_routing_test
                 tests/integration/ipc_generation_routing_test.cpp)
  target_link_libraries(fcitx5_ipc_generation_routing_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_ipc_generation_routing_test)
  fcitx_apply_binary_hardening(fcitx5_ipc_generation_routing_test)
  add_test(NAME ipc-generation-routing
           COMMAND fcitx5_ipc_generation_routing_test $<TARGET_FILE:fcitx5_mock_engine>)
  set_tests_properties(ipc-generation-routing PROPERTIES TIMEOUT 30)

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

  add_executable(fcitx5_crash_engine_fixture tests/fixtures/crashing_engine_main.cpp)
  set_target_properties(
    fcitx5_crash_engine_fixture PROPERTIES OUTPUT_NAME "fcitx5-crash-engine-fixture")
  fcitx_apply_project_options(fcitx5_crash_engine_fixture)
  fcitx_apply_binary_hardening(fcitx5_crash_engine_fixture)
  add_executable(fcitx5_launcher_crash_loop_test
                 tests/integration/launcher_crash_loop_test.cpp)
  target_link_libraries(
    fcitx5_launcher_crash_loop_test PRIVATE fcitx5::ipc_client fcitx5::launcher_state)
  fcitx_apply_project_options(fcitx5_launcher_crash_loop_test)
  fcitx_apply_binary_hardening(fcitx5_launcher_crash_loop_test)
  add_test(NAME launcher-crash-loop-safe-mode
           COMMAND fcitx5_launcher_crash_loop_test
                   $<TARGET_FILE:fcitx5_launcher>
                   $<TARGET_FILE:fcitx5_crash_engine_fixture>)

  add_executable(fcitx5_tsf_module_test tests/unit/tsf_module_test.cpp)
  target_sources(fcitx5_tsf_module_test PRIVATE src/tsf/input_profiles.cpp)
  target_include_directories(fcitx5_tsf_module_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
  target_link_libraries(fcitx5_tsf_module_test PRIVATE fcitx5::release_identity uuid ole32)
  fcitx_apply_project_options(fcitx5_tsf_module_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_module_test)
  add_test(NAME tsf-module-activation
           COMMAND fcitx5_tsf_module_test $<TARGET_FILE:fcitx5_tsf>)

  add_executable(fcitx5_input_scope_policy_test tests/unit/input_scope_policy_test.cpp)
  target_include_directories(
    fcitx5_input_scope_policy_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
  fcitx_apply_project_options(fcitx5_input_scope_policy_test)
  fcitx_apply_binary_hardening(fcitx5_input_scope_policy_test)
  add_test(NAME tsf-sensitive-input-policy COMMAND fcitx5_input_scope_policy_test)

  add_executable(fcitx5_tsf_key_commit_test tests/integration/tsf_key_commit_test.cpp)
  target_sources(fcitx5_tsf_key_commit_test PRIVATE src/tsf/input_scope_guids.cpp)
  target_include_directories(fcitx5_tsf_key_commit_test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
  target_link_libraries(fcitx5_tsf_key_commit_test PRIVATE fcitx5::release_identity uuid ole32 oleaut32)
  fcitx_apply_project_options(fcitx5_tsf_key_commit_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_key_commit_test)
  add_test(NAME tsf-key-commit-e2e
           COMMAND fcitx5_tsf_key_commit_test
                   $<TARGET_FILE:fcitx5_tsf>
                   $<TARGET_FILE:fcitx5_mock_engine>)

  # Interactive desktop E2E. It is built with the test suite but intentionally
  # not registered with CTest because it requires installed TSF registration
  # and a foreground user desktop with Notepad.
  add_executable(fcitx5_tsf_notepad_e2e_test
                 tests/integration/tsf_notepad_e2e_test.cpp)
  target_sources(fcitx5_tsf_notepad_e2e_test PRIVATE src/tsf/input_profiles.cpp)
  target_include_directories(fcitx5_tsf_notepad_e2e_test
                             PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
  target_link_libraries(fcitx5_tsf_notepad_e2e_test
                        PRIVATE fcitx5::release_identity ole32 user32)
  fcitx_apply_project_options(fcitx5_tsf_notepad_e2e_test)
  fcitx_apply_binary_hardening(fcitx5_tsf_notepad_e2e_test)

  add_executable(fcitx5_engine_integration_test
                 tests/integration/fcitx_engine_integration_test.cpp)
  target_link_libraries(fcitx5_engine_integration_test PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_engine_integration_test)
  fcitx_apply_binary_hardening(fcitx5_engine_integration_test)
endif()

if(FCITX_BUILD_FUZZERS)
  add_executable(fcitx5_protocol_fuzz tests/fuzz/protocol_fuzz.cpp)
  target_link_libraries(fcitx5_protocol_fuzz PRIVATE fcitx5::protocol)
  fcitx_apply_project_options(fcitx5_protocol_fuzz)
  fcitx_apply_binary_hardening(fcitx5_protocol_fuzz)
  if(BUILD_TESTING)
    add_test(NAME protocol-fuzz-smoke COMMAND fcitx5_protocol_fuzz)
  endif()

  add_executable(fcitx5_package_fuzz tests/fuzz/package_fuzz.cpp)
  target_include_directories(
    fcitx5_package_fuzz SYSTEM PRIVATE "${FCITX_MINIZ_SOURCE_DIR}")
  target_link_libraries(fcitx5_package_fuzz PRIVATE fcitx5::package_core fcitx5_miniz)
  fcitx_apply_project_options(fcitx5_package_fuzz)
  fcitx_apply_binary_hardening(fcitx5_package_fuzz)
  if(BUILD_TESTING)
    add_test(NAME package-manifest-path-fuzz-smoke COMMAND fcitx5_package_fuzz)
  endif()
endif()

if(FCITX_BUILD_BENCHMARKS)
  add_executable(fcitx5_focus_context_churn tests/perf/focus_context_churn.cpp)
  target_link_libraries(fcitx5_focus_context_churn PRIVATE fcitx5::candidate_model)
  fcitx_apply_project_options(fcitx5_focus_context_churn)
  fcitx_apply_binary_hardening(fcitx5_focus_context_churn)

  add_executable(fcitx5_handle_leak_soak tests/perf/handle_leak_soak.cpp)
  target_include_directories(
    fcitx5_handle_leak_soak PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/src/tsf")
  target_link_libraries(fcitx5_handle_leak_soak PRIVATE fcitx5::release_identity uuid ole32 user32)
  fcitx_apply_project_options(fcitx5_handle_leak_soak)
  fcitx_apply_binary_hardening(fcitx5_handle_leak_soak)

  add_executable(fcitx5_candidate_render_bench tests/perf/candidate_render_bench.cpp)
  target_link_libraries(
    fcitx5_candidate_render_bench
    PRIVATE fcitx5::candidate_model d2d1 dwrite user32)
  fcitx_apply_project_options(fcitx5_candidate_render_bench)
  fcitx_apply_binary_hardening(fcitx5_candidate_render_bench)

  add_executable(fcitx5_candidate_model_bench tests/perf/candidate_model_bench.cpp)
  target_link_libraries(fcitx5_candidate_model_bench PRIVATE fcitx5::candidate_model)
  fcitx_apply_project_options(fcitx5_candidate_model_bench)
  fcitx_apply_binary_hardening(fcitx5_candidate_model_bench)

  add_executable(fcitx5_ipc_codec_bench tests/perf/ipc_codec_bench.cpp)
  target_link_libraries(fcitx5_ipc_codec_bench PRIVATE fcitx5::protocol)
  fcitx_apply_project_options(fcitx5_ipc_codec_bench)
  fcitx_apply_binary_hardening(fcitx5_ipc_codec_bench)

  add_executable(fcitx5_key_roundtrip_bench tests/perf/key_roundtrip_bench.cpp)
  target_link_libraries(fcitx5_key_roundtrip_bench PRIVATE fcitx5::ipc_client)
  fcitx_apply_project_options(fcitx5_key_roundtrip_bench)
  fcitx_apply_binary_hardening(fcitx5_key_roundtrip_bench)
endif()
