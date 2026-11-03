// The desktop program of C++ applications (DESIGN 7.9): Clarity.UI draws
// the windows in the program's own process, and asks the program by command
// (clarity_ui.h). The service answers the commands of the screen's wire;
// this adds the desktop's own: how it starts, the facts of the fields, and
// the watches of long calls, which come from several threads.

#pragma once

#include <atomic>
#include <cstdint>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <string_view>

#include "shell/shell.hpp"
#include "ui/ui.hpp"

namespace clarity::shell {

/// The program as Clarity.UI asks it, answered by the service.
class DesktopProgram final : public ui::Program {
public:
    explicit DesktopProgram(Options options);

    std::optional<std::string> call(std::string_view command, std::string_view args) override;

    Service& service() { return service_; }

private:
    /// A long call's watch, by the number the screen picked.
    struct Watch;
    std::shared_ptr<Watch> watch(uint64_t id);
    nlohmann::json watching(const std::string& command, uint64_t id);

    Service service_;
    std::mutex watches_mutex_;
    std::map<uint64_t, std::shared_ptr<Watch>> watches_;
};

/// The files Clarity.UI draws from: the options' windows, app frames,
/// themes and resources, the model, the title and the main window's size.
ui::Files desktop_files(const Options& options);

/// Shows the application's windows and runs until the last one closed; the exit code.
int run(const Options& options);

}  // namespace clarity::shell
