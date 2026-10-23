// Custom parts in the shell (DESIGN 6.5): the native parts of a program are
// in libclarity_parts beside it, which the shell loads when it is there and
// asks to place, set and tell its parts. A program without native parts has
// no such library, and nothing here does anything.

#pragma once

#include <cstdint>
#include <functional>
#include <string>

namespace clarity::shell::parts {

/// What a part told: its window's number, the part, the tell and its values as JSON.
using Told = std::function<void(uint64_t window, const std::string& part, const std::string& tell, const std::string& values)>;

/// Whether the program has native parts: libclarity_parts beside it.
bool present();

/// Loads and starts them; empty, or why they cannot run. `told` is called on the parts' own thread.
std::string start(Told told);

/// Whether they run.
bool running();

void place(uint64_t window, const std::string& part, const std::string& kind, uintptr_t parent, int x, int y, int width, int height, bool shown);
void set(uint64_t window, const std::string& part, const std::string& name, const std::string& value);
void hear(uint64_t window, const std::string& part, const std::string& hear, const std::string& values);
void close(uint64_t window);
void stop();

}  // namespace clarity::shell::parts
