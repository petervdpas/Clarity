# Clarity.UI's library for a desktop program (DESIGN 7.9) when CLARITY_UI
# names no folder: the archive of the version the project names
# (CLARITY_UI_VERSION) is downloaded from the library's release on GitHub,
# checked against the release's SHA256SUMS and unpacked into the build
# folder, once per version. Clarity's own build names the folder it
# fetched, so this is for a build outside Clarity. The C and C++ runtimes
# carry the same file.
if(NOT DEFINED CLARITY_UI_RELEASES)
  set(CLARITY_UI_RELEASES "https://github.com/petervdpas/Clarity/releases/download")
endif()

# The release's name for a system (CMAKE_SYSTEM_NAME) and processor
# (CMAKE_SYSTEM_PROCESSOR): linux-x64, win-arm64, osx-arm64; empty for a
# system the release has no archive for.
function(clarity_ui_rid out system processor)
  string(TOLOWER "${processor}" processor)
  set(rid "")
  if(system STREQUAL "Linux")
    set(os linux)
  elseif(system STREQUAL "Windows")
    set(os win)
  elseif(system STREQUAL "Darwin")
    set(os osx)
  endif()
  if(processor MATCHES "^(x86_64|amd64|x64)$")
    set(arch x64)
  elseif(processor MATCHES "^(aarch64|arm64)$")
    set(arch arm64)
  endif()
  if(os AND arch)
    set(rid "${os}-${arch}")
  endif()
  set(${out} "${rid}" PARENT_SCOPE)
endfunction()

# Fetches the library `version` for `rid` from `releases` into `folder`,
# unless it is there already, and sets `out` to the unpacked folder
# (`folder`/clarity-ui-<version>-<rid>, with include/ and lib/).
function(clarity_ui_fetch out releases version rid folder)
  set(name clarity-ui-${version}-${rid})
  set(into ${folder}/${name})
  if(NOT EXISTS ${into}/include/clarity_ui.h)
    set(release ${releases}/runtimes/rust-ui-ffi/v${version})
    message(STATUS "Clarity.UI's library ${version} for ${rid}: fetching ${release}/${name}.tar.gz")
    file(DOWNLOAD ${release}/SHA256SUMS ${folder}/SHA256SUMS STATUS status)
    list(GET status 0 code)
    if(NOT code EQUAL 0)
      message(FATAL_ERROR "cannot download ${release}/SHA256SUMS: ${status}")
    endif()
    # Lines as sha256sum writes them: the hash, then the file's name.
    file(STRINGS ${folder}/SHA256SUMS lines)
    set(hash "")
    foreach(line IN LISTS lines)
      if(line MATCHES "^([0-9a-fA-F]+) +\\*?(.+)$" AND CMAKE_MATCH_2 STREQUAL "${name}.tar.gz")
        set(hash ${CMAKE_MATCH_1})
      endif()
    endforeach()
    if(NOT hash)
      message(FATAL_ERROR "the release's SHA256SUMS names no ${name}.tar.gz")
    endif()
    file(DOWNLOAD ${release}/${name}.tar.gz ${folder}/${name}.tar.gz EXPECTED_HASH SHA256=${hash} STATUS status)
    list(GET status 0 code)
    if(NOT code EQUAL 0)
      file(REMOVE ${folder}/${name}.tar.gz)
      message(FATAL_ERROR "cannot download ${release}/${name}.tar.gz: ${status}")
    endif()
    file(ARCHIVE_EXTRACT INPUT ${folder}/${name}.tar.gz DESTINATION ${folder})
    file(REMOVE ${folder}/${name}.tar.gz ${folder}/SHA256SUMS)
    if(NOT EXISTS ${into}/include/clarity_ui.h)
      message(FATAL_ERROR "the archive ${name}.tar.gz has no ${name}/include/clarity_ui.h")
    endif()
  endif()
  set(${out} ${into} PARENT_SCOPE)
endfunction()
