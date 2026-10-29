#include "clarity/watch.hpp"

namespace clarity {

namespace {
thread_local const EngineWatch* current = nullptr;
}

const EngineWatch* current_watch() { return current; }

Watching::Watching(const EngineWatch& watch) : was_(current) { current = &watch; }

Watching::~Watching() { current = was_; }

}  // namespace clarity
