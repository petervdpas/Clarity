#include "shell/parts.hpp"

#include <climits>
#include <string>

#if defined(__linux__) || defined(__APPLE__)
#include <dlfcn.h>
#include <unistd.h>
#endif

#if defined(_WIN32)
#include <windows.h>
#endif

namespace clarity::shell::parts {

namespace {

using TellFn = void (*)(uint64_t, const char*, const char*, const char*, void*);
using StartFn = bool (*)(TellFn, void*);
using ProblemFn = const char* (*)();
using StopFn = void (*)();
using PlaceFn = void (*)(uint64_t, const char*, const char*, uintptr_t, int, int, int, int, bool);
using SetFn = void (*)(uint64_t, const char*, const char*, const char*);
using CloseFn = void (*)(uint64_t);

struct Library {
    StartFn start = nullptr;
    ProblemFn problem = nullptr;
    StopFn stop = nullptr;
    PlaceFn place = nullptr;
    SetFn set = nullptr;
    SetFn hear = nullptr;
    CloseFn close = nullptr;
};

Library library;
bool started = false;
Told told_to;

/// libclarity_parts beside the program.
std::string beside_program() {
#if defined(__linux__)
    char path[PATH_MAX];
    ssize_t n = readlink("/proc/self/exe", path, sizeof path - 1);
    if (n <= 0) return {};
    std::string exe(path, static_cast<size_t>(n));
    return exe.substr(0, exe.find_last_of('/') + 1) + "libclarity_parts.so";
#elif defined(_WIN32)
    char path[MAX_PATH];
    DWORD n = GetModuleFileNameA(nullptr, path, MAX_PATH);
    if (n == 0 || n >= MAX_PATH) return {};
    std::string exe(path, n);
    size_t at = exe.find_last_of("\\/");
    return exe.substr(0, at == std::string::npos ? 0 : at + 1) + "clarity_parts.dll";
#else
    return {};
#endif
}

/// A function of the loaded part library, by name.
void* find_in(void* handle, const char* name) {
#if defined(_WIN32)
    return reinterpret_cast<void*>(GetProcAddress(static_cast<HMODULE>(handle), name));
#elif defined(__linux__) || defined(__APPLE__)
    return dlsym(handle, name);
#else
    (void)handle, (void)name;
    return nullptr;
#endif
}

void on_told(uint64_t window, const char* part, const char* tell, const char* values, void*) {
    if (told_to) told_to(window, part, tell, values);
}

}  // namespace

bool present() {
#if defined(__linux__) || defined(__APPLE__)
    std::string path = beside_program();
    return !path.empty() && access(path.c_str(), R_OK) == 0;
#elif defined(_WIN32)
    std::string path = beside_program();
    return !path.empty() && GetFileAttributesA(path.c_str()) != INVALID_FILE_ATTRIBUTES;
#else
    return false;
#endif
}

std::string start(Told told) {
#if defined(__linux__) || defined(__APPLE__) || defined(_WIN32)
    if (started) return {};
    std::string path = beside_program();
    if (path.empty()) return "the program's custom parts cannot be found";
#if defined(_WIN32)
    // Windows loads what is beside the program; Linux and macOS dlopen it.
    void* handle = reinterpret_cast<void*>(LoadLibraryA(path.c_str()));
    if (!handle) return "the program's custom parts cannot load: " + path;
#else
    void* handle = dlopen(path.c_str(), RTLD_NOW | RTLD_LOCAL);
    if (!handle) return std::string("the program's custom parts cannot load: ") + dlerror();
#endif
    auto find = [&](const char* name) { return find_in(handle, name); };
    library.start = reinterpret_cast<StartFn>(find("clarity_parts_start"));
    library.problem = reinterpret_cast<ProblemFn>(find("clarity_parts_problem"));
    library.stop = reinterpret_cast<StopFn>(find("clarity_parts_stop"));
    library.place = reinterpret_cast<PlaceFn>(find("clarity_parts_place"));
    library.set = reinterpret_cast<SetFn>(find("clarity_parts_set"));
    library.hear = reinterpret_cast<SetFn>(find("clarity_parts_hear"));
    library.close = reinterpret_cast<CloseFn>(find("clarity_parts_close"));
    if (!library.start || !library.problem || !library.stop || !library.place || !library.set || !library.hear || !library.close) {
        return "the program's custom parts library is not Clarity's";
    }
    told_to = std::move(told);
    if (!library.start(on_told, nullptr)) return library.problem();
    started = true;
    return {};
#else
    (void)told;
    return "custom parts drawn by a native library run on Linux for now";
#endif
}

bool running() { return started; }

void place(uint64_t window, const std::string& part, const std::string& kind, uintptr_t parent, int x, int y, int width, int height, bool shown) {
    if (started) library.place(window, part.c_str(), kind.c_str(), parent, x, y, width, height, shown);
}

void set(uint64_t window, const std::string& part, const std::string& name, const std::string& value) {
    if (started) library.set(window, part.c_str(), name.c_str(), value.c_str());
}

void hear(uint64_t window, const std::string& part, const std::string& name, const std::string& values) {
    if (started) library.hear(window, part.c_str(), name.c_str(), values.c_str());
}

void close(uint64_t window) {
    if (started) library.close(window);
}

void stop() {
    if (started) library.stop();
    started = false;
}

}  // namespace clarity::shell::parts
