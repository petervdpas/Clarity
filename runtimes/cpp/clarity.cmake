# The runtime's libraries, for the runtime's own build and for generated
# projects, which include this file: `clarity` (records, storage, the
# binding to the screen) and `clarity_shell` (the window and webview).
find_package(SQLite3 REQUIRED)
find_package(nlohmann_json 3 REQUIRED)

file(GLOB CLARITY_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_LIST_DIR}/clarity/*.cpp)
add_library(clarity STATIC ${CLARITY_SOURCES})
target_include_directories(clarity PUBLIC ${CMAKE_CURRENT_LIST_DIR})
target_link_libraries(clarity PUBLIC SQLite::SQLite3 nlohmann_json::nlohmann_json)
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
  target_compile_options(clarity PRIVATE -Wall -Wextra)
endif()

# The service, which answers the renderer's commands and serves it to a
# browser (the web target): nothing of the desktop toolkit.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/service.cpp)
  add_library(clarity_service STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/service.cpp ${CMAKE_CURRENT_LIST_DIR}/shell/serve.cpp)
  target_include_directories(clarity_service PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_service PUBLIC clarity)
  if(WIN32)
    # Winsock for the server, the shell for links.
    target_link_libraries(clarity_service PUBLIC ws2_32 shell32)
  endif()
endif()

# The shell, which shows the windows: only in a project whose runtime copy
# has it, so a command-line program and a web server build without a webview.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/shell.cpp)
  # The webview draws with WebKitGTK on Linux, WebView2 on Windows and WebKit on macOS.
  add_library(clarity_shell STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/shell.cpp ${CMAKE_CURRENT_LIST_DIR}/shell/parts.cpp)
  target_include_directories(clarity_shell PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
  target_link_libraries(clarity_shell PUBLIC clarity_service)
  # A handler runs on a thread of its own, so the window keeps painting (DESIGN 5.12).
  find_package(Threads REQUIRED)
  target_link_libraries(clarity_shell PUBLIC Threads::Threads)
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
    target_link_libraries(clarity_shell PUBLIC advapi32 ole32 shell32 shlwapi user32 uuid version)
  endif()
endif()

# HTTP calls from code (DESIGN 5.23), through libcurl: only in a project
# whose runtime copy has them, that is, one whose code calls out.
# Secret settings (DESIGN 5.27) in the system's protected store: only in a
# project whose runtime copy has them, that is, one with a secret setting.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.cpp)
  add_library(clarity_secrets STATIC ${CMAKE_CURRENT_LIST_DIR}/secrets/secrets.cpp)
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
  if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_secrets PRIVATE -Wall -Wextra)
  endif()
endif()

if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/http/http.cpp)
  find_package(CURL REQUIRED)
  add_library(clarity_http STATIC ${CMAKE_CURRENT_LIST_DIR}/http/http.cpp)
  target_include_directories(clarity_http PUBLIC ${CMAKE_CURRENT_LIST_DIR})
  target_link_libraries(clarity_http PUBLIC clarity CURL::libcurl)
  if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(clarity_http PRIVATE -Wall -Wextra)
  endif()
endif()

# Engines (DESIGN 5.11), hosted by wasm3: only in a project whose runtime
# copy has them, that is, one that declares an engine. wasm3 is C; the
# runtime's own build takes it from the C runtime beside it.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/engine/engine.cpp)
  enable_language(C)
  set(WASM3_DIR ${CMAKE_CURRENT_LIST_DIR}/third_party/wasm3)
  if(NOT EXISTS ${WASM3_DIR})
    set(WASM3_DIR ${CMAKE_CURRENT_LIST_DIR}/../c/third_party/wasm3)
  endif()
  file(GLOB WASM3_SOURCES CONFIGURE_DEPENDS ${WASM3_DIR}/*.c)
  add_library(clarity_engine STATIC ${CMAKE_CURRENT_LIST_DIR}/engine/engine.cpp ${WASM3_SOURCES})
  target_include_directories(clarity_engine PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${WASM3_DIR})
  target_link_libraries(clarity_engine PUBLIC clarity m)
  # wasm3 is taken as released; its own warnings are not the runtime's.
  set_source_files_properties(${WASM3_SOURCES} PROPERTIES COMPILE_OPTIONS "-w")
  if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    set_source_files_properties(${CMAKE_CURRENT_LIST_DIR}/engine/engine.cpp PROPERTIES COMPILE_OPTIONS "-Wall;-Wextra")
  endif()
endif()
