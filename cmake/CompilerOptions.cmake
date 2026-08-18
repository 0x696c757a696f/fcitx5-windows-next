function(fcitx_apply_project_options target)
  target_compile_features(${target} PRIVATE cxx_std_20)
  target_compile_definitions(
    ${target}
    PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0601)

  if(MSVC)
    target_compile_options(
      ${target}
      PRIVATE /W4 /permissive- /sdl /Zc:__cplusplus /utf-8 /EHsc)

    if(FCITX_WARNINGS_AS_ERRORS)
      target_compile_options(${target} PRIVATE /WX)
    endif()

    if(FCITX_ENABLE_MSVC_ANALYZE)
      target_compile_options(${target} PRIVATE /analyze)
    endif()
  else()
    target_compile_options(
      ${target}
      PRIVATE -Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion)
    if(FCITX_WARNINGS_AS_ERRORS)
      target_compile_options(${target} PRIVATE -Werror)
    endif()
  endif()
endfunction()

function(fcitx_apply_binary_hardening target)
  if(MSVC)
    target_compile_options(${target} PRIVATE /guard:cf)
    target_link_options(
      ${target}
      PRIVATE /DYNAMICBASE /NXCOMPAT /guard:cf)
    # /CETCOMPAT (CET shadow stack) is only supported for x64 targets; the
    # ARM64 linker rejects it with LNK1246. Keep the hardening flags that
    # apply everywhere and add CET only on x64.
    if(CMAKE_SIZEOF_VOID_P EQUAL 8 AND NOT CMAKE_CROSSCOMPILING_EMULATOR AND
       CMAKE_SYSTEM_PROCESSOR MATCHES "AMD64|x86_64")
      target_link_options(${target} PRIVATE /CETCOMPAT)
    endif()
  endif()
endfunction()
