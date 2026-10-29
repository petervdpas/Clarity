// What the screen sees of a long engine call and how the person stops it
// (DESIGN 5.12, `uses progress`): the shell watches the handler it runs,
// and every engine call the handler makes reports to that watch.

#pragma once

#include <atomic>
#include <cstdint>
#include <functional>
#include <memory>

namespace clarity {

/// What the screen sees of a long call and how the person stops it (`uses
/// progress`, conformance/progress.json): every report of how far it is
/// goes to `progress`, and once `stop` is set (from any thread) the call
/// ends, failing with "it was stopped". Either may be empty.
struct EngineWatch {
    std::function<void(int64_t done, int64_t total)> progress;
    std::shared_ptr<std::atomic<bool>> stop;
};

/// The watch of the handler running on this thread; none outside one.
const EngineWatch* current_watch();

/// Makes a watch that of every engine call this thread makes while it lives,
/// as a shell does around a handler.
class Watching {
public:
    explicit Watching(const EngineWatch& watch);
    ~Watching();
    Watching(const Watching&) = delete;
    Watching& operator=(const Watching&) = delete;

private:
    const EngineWatch* was_;
};

}  // namespace clarity
