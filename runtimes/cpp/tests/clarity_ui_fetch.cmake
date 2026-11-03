# Tests clarity_ui.cmake, run as a script (cmake -DWORK=<folder> -P ...):
# the release's name per system, and a fetch from a release made here with
# file:// URLs, checked against its SHA256SUMS. With -DMODE=bad it fetches
# from a release whose SHA256SUMS does not match, which must fail.
include(${CMAKE_CURRENT_LIST_DIR}/../clarity_ui.cmake)

function(expect actual expected)
  if(NOT "${actual}" STREQUAL "${expected}")
    message(FATAL_ERROR "expected '${expected}', got '${actual}'")
  endif()
endfunction()

if(WORK MATCHES "^/")
  set(releases "file://${WORK}/releases")
else()
  set(releases "file:///${WORK}/releases")
endif()
set(name clarity-ui-9.9.9-linux-x64)
set(release ${WORK}/releases/runtimes/rust-ui-ffi/v9.9.9)

if(MODE STREQUAL "bad")
  file(WRITE ${release}/SHA256SUMS "0000000000000000000000000000000000000000000000000000000000000000  ${name}.tar.gz\n")
  clarity_ui_fetch(folder ${releases} 9.9.9 linux-x64 ${WORK}/bad)
  return()
endif()

foreach(case "Linux;x86_64;linux-x64" "Linux;aarch64;linux-arm64" "Windows;AMD64;win-x64" "Windows;ARM64;win-arm64" "Darwin;x86_64;osx-x64" "Darwin;arm64;osx-arm64" "FreeBSD;amd64;" "Linux;riscv64;")
  list(GET case 0 system)
  list(GET case 1 processor)
  list(LENGTH case n)
  set(expected "")
  if(n EQUAL 3)
    list(GET case 2 expected)
  endif()
  clarity_ui_rid(rid ${system} ${processor})
  expect("${rid}" "${expected}")
endforeach()

# A release as the workflow publishes it: the archive of one folder, and SHA256SUMS.
file(REMOVE_RECURSE ${WORK})
file(WRITE ${WORK}/pack/${name}/include/clarity_ui.h "/* header */\n")
file(WRITE ${WORK}/pack/${name}/lib/libclarity_ui.so "library")
file(MAKE_DIRECTORY ${release})
execute_process(COMMAND ${CMAKE_COMMAND} -E tar czf ${release}/${name}.tar.gz ${name} WORKING_DIRECTORY ${WORK}/pack RESULT_VARIABLE packed)
expect("${packed}" "0")
file(SHA256 ${release}/${name}.tar.gz hash)
file(WRITE ${release}/SHA256SUMS "1111111111111111111111111111111111111111111111111111111111111111  clarity-ui-9.9.9-win-x64.tar.gz\n${hash}  ${name}.tar.gz\n")

clarity_ui_fetch(folder ${releases} 9.9.9 linux-x64 ${WORK}/build)
expect("${folder}" "${WORK}/build/${name}")
file(READ ${folder}/lib/libclarity_ui.so library)
expect("${library}" "library")
if(EXISTS ${WORK}/build/${name}.tar.gz OR EXISTS ${WORK}/build/SHA256SUMS)
  message(FATAL_ERROR "the download is left in the build folder")
endif()

# Fetched once: the second time needs no release.
file(REMOVE_RECURSE ${WORK}/releases)
clarity_ui_fetch(again ${releases} 9.9.9 linux-x64 ${WORK}/build)
expect("${again}" "${folder}")

# An archive that is not the release's is refused and not unpacked.
file(MAKE_DIRECTORY ${release})
file(WRITE ${release}/${name}.tar.gz "not the release's archive")
execute_process(COMMAND ${CMAKE_COMMAND} -DWORK=${WORK} -DMODE=bad -P ${CMAKE_CURRENT_LIST_FILE} RESULT_VARIABLE refused OUTPUT_QUIET ERROR_QUIET)
if(refused EQUAL 0)
  message(FATAL_ERROR "an archive that does not match SHA256SUMS was taken")
endif()
if(EXISTS ${WORK}/bad/${name})
  message(FATAL_ERROR "an archive that does not match SHA256SUMS was unpacked")
endif()
file(REMOVE_RECURSE ${WORK})
message(STATUS "clarity_ui.cmake: all checks passed")
