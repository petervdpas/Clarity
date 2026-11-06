// What the splash screen says is loading; see splash.hpp.

#include "clarity/splash.hpp"

#include <mutex>

namespace clarity::splash {

namespace {
std::mutex gate;
std::optional<Text> said;
}  // namespace

void set_status(const Text& text) {
    std::lock_guard lock(gate);
    said = text;
}

std::optional<Text> status() {
    std::lock_guard lock(gate);
    return said;
}

}  // namespace clarity::splash
