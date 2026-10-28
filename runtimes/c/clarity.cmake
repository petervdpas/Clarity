# The runtime's libraries, for the runtime's own build and for generated
# projects, which include this file: `clarity` (records, storage, the
# binding to the screen, with cJSON) and `clarity_shell` (the window and
# webview). The webview's own code is C++, so a project needs a C++
# compiler for that one file.
find_package(SQLite3 REQUIRED)

file(GLOB CLARITY_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_LIST_DIR}/clarity/*.c)
add_library(clarity STATIC ${CLARITY_SOURCES} ${CMAKE_CURRENT_LIST_DIR}/third_party/cjson/cJSON.c)
target_include_directories(clarity PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
target_link_libraries(clarity PUBLIC SQLite::SQLite3 m)
if(MINGW)
  # The C99 printf, so %zu and %lld print as they do elsewhere.
  target_compile_definitions(clarity PUBLIC __USE_MINGW_ANSI_STDIO=1)
endif()
if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
  target_compile_options(clarity PRIVATE $<$<COMPILE_LANGUAGE:C>:-Wall -Wextra>)
endif()

# The shell, which shows the windows: only in a project whose runtime copy
# has it, so a command-line program builds without a webview.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/service.c)
  # The service answers the renderer's commands, and serves it to a browser
  # (the web target): nothing of the desktop toolkit.
  add_library(clarity_service STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/service.c ${CMAKE_CURRENT_LIST_DIR}/shell/serve.c)
  target_include_directories(clarity_service PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_service PUBLIC clarity)
  if(WIN32)
    # Winsock for the server, the shell for links.
    target_link_libraries(clarity_service PUBLIC ws2_32 shell32)
  endif()
endif()

if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/shell.c)
  # The webview draws with WebKitGTK on Linux, WebView2 on Windows and WebKit on macOS.
  add_library(clarity_shell STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/shell.c ${CMAKE_CURRENT_LIST_DIR}/shell/parts.c ${CMAKE_CURRENT_LIST_DIR}/shell/webview.cc)
  target_include_directories(clarity_shell PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
  target_compile_definitions(clarity_shell PUBLIC WEBVIEW_STATIC)
  target_compile_features(clarity_shell PRIVATE cxx_std_11)
  target_link_libraries(clarity_shell PUBLIC clarity_service)
  if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(WEBKIT REQUIRED IMPORTED_TARGET gtk+-3.0 webkit2gtk-4.1)
    target_link_libraries(clarity_shell PUBLIC PkgConfig::WEBKIT dl)
  elseif(APPLE)
    target_link_libraries(clarity_shell PUBLIC "-framework WebKit")
  elseif(WIN32)
    # The webview draws with WebView2, and builds against Microsoft's SDK
    # headers, which are not packaged for MinGW or MSYS2. CMake fetches the
    # SDK once into the build folder and keeps it there; the loader itself is
    # built into webview.h, so no import library is needed, and the WebView2
    # runtime ships with Windows 11.
    set(WEBVIEW2_VERSION 1.0.2903.40)
    set(WEBVIEW2_DIR ${CMAKE_BINARY_DIR}/webview2)
    if(NOT EXISTS ${WEBVIEW2_DIR}/build/native/include/WebView2.h)
      message(STATUS "Fetching the WebView2 SDK ${WEBVIEW2_VERSION}")
      file(DOWNLOAD
        https://www.nuget.org/api/v2/package/Microsoft.Web.WebView2/${WEBVIEW2_VERSION}
        ${CMAKE_BINARY_DIR}/webview2.nupkg
        STATUS WEBVIEW2_FETCHED)
      list(GET WEBVIEW2_FETCHED 0 WEBVIEW2_FAILED)
      if(WEBVIEW2_FAILED)
        list(GET WEBVIEW2_FETCHED 1 WEBVIEW2_WHY)
        message(FATAL_ERROR "the WebView2 SDK could not be fetched (${WEBVIEW2_WHY}); download Microsoft.Web.WebView2 and point WEBVIEW2_DIR at it")
      endif()
      file(ARCHIVE_EXTRACT INPUT ${CMAKE_BINARY_DIR}/webview2.nupkg DESTINATION ${WEBVIEW2_DIR})
    endif()
    target_include_directories(clarity_shell PUBLIC ${WEBVIEW2_DIR}/build/native/include)
    target_link_libraries(clarity_shell PUBLIC advapi32 ole32 shell32 shlwapi user32 version)
  endif()
endif()

# HTTP calls from code (DESIGN 5.23), through libcurl: only in a project
# whose runtime copy has them, that is, one whose code calls out.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/http/http.c)
  find_package(CURL REQUIRED)
  add_library(clarity_http STATIC ${CMAKE_CURRENT_LIST_DIR}/http/http.c)
  target_include_directories(clarity_http PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_http PUBLIC clarity CURL::libcurl)
  if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_http PRIVATE -Wall -Wextra)
  endif()
endif()

# Secret settings (DESIGN 5.27) in the system's protected store: only in a
# project whose runtime copy has them, that is, one with a secret setting.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.c)
  add_library(clarity_secrets STATIC ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.c)
  target_include_directories(clarity_secrets PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_secrets PUBLIC clarity)
  if(WIN32)
    target_link_libraries(clarity_secrets PUBLIC advapi32)
  elseif(APPLE)
    target_link_libraries(clarity_secrets PUBLIC "-framework Security" "-framework CoreFoundation")
  else()
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(CLARITY_GIO REQUIRED IMPORTED_TARGET gio-2.0)
    target_link_libraries(clarity_secrets PUBLIC PkgConfig::CLARITY_GIO)
  endif()
  if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_secrets PRIVATE -Wall -Wextra)
  endif()
endif()

# Engines (DESIGN 5.11), hosted by wasm3: only in a project whose runtime
# copy has them, that is, one that declares an engine.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/engine/engine.c)
  file(GLOB WASM3_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_LIST_DIR}/third_party/wasm3/*.c)
  add_library(clarity_engine STATIC ${CMAKE_CURRENT_LIST_DIR}/engine/engine.c ${WASM3_SOURCES})
  target_include_directories(clarity_engine PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
  target_link_libraries(clarity_engine PUBLIC clarity m)
  # wasm3 is taken as released; its own warnings are not the runtime's.
  set_source_files_properties(${WASM3_SOURCES} PROPERTIES COMPILE_OPTIONS "-w")
  if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    set_source_files_properties(${CMAKE_CURRENT_LIST_DIR}/engine/engine.c PROPERTIES COMPILE_OPTIONS "-Wall;-Wextra")
  endif()
endif()
