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

# The shell, which shows the windows: only in a project whose runtime copy
# has it, so a command-line program builds without a webview.
if(EXISTS ${CMAKE_CURRENT_LIST_DIR}/shell/shell.cpp)
  # The webview draws with WebKitGTK on Linux, WebView2 on Windows and WebKit on macOS.
  add_library(clarity_shell STATIC ${CMAKE_CURRENT_LIST_DIR}/shell/shell.cpp)
  target_include_directories(clarity_shell PUBLIC ${CMAKE_CURRENT_LIST_DIR} ${CMAKE_CURRENT_LIST_DIR}/third_party)
  target_link_libraries(clarity_shell PUBLIC clarity)
  if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(WEBKIT REQUIRED IMPORTED_TARGET gtk+-3.0 webkit2gtk-4.1)
    target_link_libraries(clarity_shell PUBLIC PkgConfig::WEBKIT dl)
  elseif(APPLE)
    target_link_libraries(clarity_shell PUBLIC "-framework WebKit")
  elseif(WIN32)
    target_link_libraries(clarity_shell PUBLIC advapi32 ole32 shell32 shlwapi user32 version)
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
