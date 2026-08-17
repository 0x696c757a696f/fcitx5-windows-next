cmake_minimum_required(VERSION 3.28)

project(
  fcitx5-windows-next
  VERSION 0.1.0
  DESCRIPTION "Native Fcitx5 platform frontend for Windows"
  LANGUAGES CXX)

include(CTest)

option(FCITX_WARNINGS_AS_ERRORS "Treat project warnings as errors" ON)
option(FCITX_ENABLE_MSVC_ANALYZE "Run MSVC static analysis" OFF)

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

if(BUILD_TESTING)
  add_executable(fcitx5_version_test tests/unit/version_test.cpp)
  target_link_libraries(fcitx5_version_test PRIVATE fcitx5::windows_common)
  fcitx_apply_project_options(fcitx5_version_test)
  fcitx_apply_executable_hardening(fcitx5_version_test)
  add_test(NAME version-contract COMMAND fcitx5_version_test)
endif()
