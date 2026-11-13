// Clarity.UI for C++ (DESIGN 7.9): a thin layer over the library's C
// interface (clarity_ui.h). Handles free themselves, text goes in as
// std::string_view and comes back as std::string, and a failing call throws
// clarity::ui::Error with the library's message. The program is an abstract
// class answering the library's commands; a custom part is an abstract class
// drawing on the canvas the library gives.

#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>

#include "clarity_ui.h"

namespace clarity::ui {

/// A call of the library that did not succeed: its status and its message.
class Error : public std::runtime_error {
public:
    Error(ClarityStatus status, const std::string& message) : std::runtime_error(message), status_(status) {}
    ClarityStatus status() const { return status_; }

private:
    ClarityStatus status_;
};

/// Throws the library's last error when `status` is not CLARITY_OK.
void check(ClarityStatus status);

/// A string the library gave, taken over and freed; empty for NULL.
std::string take(char* text);

/// Asks the AI agent whose command the program runs now (inside `agent_call`,
/// docs/ledger/app-mcp.md) `question` of the kind `form`: the answer as the
/// terminal gives it, or nothing.
std::optional<std::string> agent_ask(const std::string& question, const std::string& form);

/// The library's version, and whether its interface is the header's.
std::string version();
bool fits();

/// The program's files: its windows' and apps' markup, themes, model, resources, title and size.
class Files {
public:
    Files();
    Files(const Files&) = delete;
    Files& operator=(const Files&) = delete;
    Files(Files&&) noexcept;
    Files& operator=(Files&&) noexcept;
    ~Files();

    Files& window(std::string_view name, std::string_view markup);
    Files& app(std::string_view name, std::string_view markup);
    Files& theme(std::string_view name, std::string_view json);
    Files& model(std::string_view json);
    Files& resource(std::string_view name, std::string_view bytes);
    Files& title(std::string_view title);
    Files& size(double width, double height);

    const ClarityFiles* handle() const { return files_; }

private:
    ClarityFiles* files_ = nullptr;
};

/// The program as the library asks it (clarity_ui.h, "The program"): a
/// command with its arguments as JSON, answered `{"value": ...}` or
/// `{"error": ...}`, or none for the library's default. Called from the
/// screen's thread and from the threads that run handlers.
class Program {
public:
    virtual ~Program() = default;
    virtual std::optional<std::string> call(std::string_view command, std::string_view args) = 0;
};

/// The library's hold of a Program, which must outlive it.
class ProgramHandle {
public:
    explicit ProgramHandle(Program& program);
    ProgramHandle(const ProgramHandle&) = delete;
    ProgramHandle& operator=(const ProgramHandle&) = delete;
    ~ProgramHandle();

    const ClarityProgram* handle() const { return program_; }

private:
    ClarityProgram* program_ = nullptr;
};

/// Runs the program's windows in desktop windows until they closed, on this
/// thread (the main thread on macOS); driven from standard input when
/// CLARITY_HEADLESS is set. The exit code.
int run_desktop(Program& program, const Files& files);

/// Wakes the running desktop from any thread: every window reads its values again.
void wake_desktop();

/// The program on a screen without a window, driven step by step (DESIGN 1.6),
/// on the thread that made it.
class Driven {
public:
    Driven(Program& program, const Files& files);
    Driven(const Driven&) = delete;
    Driven& operator=(const Driven&) = delete;
    ~Driven();

    /// Does one step ({"do":"click","part":"save"}) and gives its answer as JSON;
    /// a step that could not be done answers with "error".
    std::string step(std::string_view step);

private:
    ProgramHandle program_;
    ClarityDriven* driven_ = nullptr;
};

// -- Custom parts (DESIGN 5.19) ----------------------------------------------

/// A color as 0xAARRGGBB.
using Color = uint32_t;

/// The canvas a custom part draws on, valid during its draw.
class Canvas {
public:
    explicit Canvas(ClarityCanvas* canvas) : canvas_(canvas) {}
    void fill(double x, double y, double width, double height, Color color, double radius = 0);
    void stroke(double x, double y, double width, double height, Color color, double line_width = 1, double radius = 0);
    void line(double x1, double y1, double x2, double y2, Color color, double line_width = 1);
    void oval(double x, double y, double width, double height, Color color, bool filled = true, double line_width = 1);
    /// An SVG path's data ("M0 0 L10 10 Z").
    void path(std::string_view svg, Color color, bool filled = true, double line_width = 1);
    /// Text in a box: align 0 left, 1 center, 2 right; size 0 is the look's.
    void text(std::string_view text, double x, double y, double width, double height, Color color, double size = 0, bool bold = false, int align = 0);
    double measure(std::string_view text, double size = 0, bool bold = false);
    void pixels(const uint8_t* rgba, uint32_t width, uint32_t height, size_t stride, double x, double y, double box_width, double box_height);
    void clip(double x, double y, double width, double height);
    void restore();
    /// A color of the look by its role ("accent", "text", "border").
    Color look(std::string_view role);

private:
    ClarityCanvas* canvas_;
};

/// What a custom part reaches of its window during a call.
class Cx {
public:
    explicit Cx(ClarityCustomCx* cx) : cx_(cx) {}
    struct Bounds {
        double x = 0, y = 0, width = 0, height = 0;
    };
    Bounds bounds() const;
    bool enabled() const;
    void invalidate();
    /// Its wake runs after `ms` milliseconds, once.
    void wake_in(double ms);
    void animate(bool on);
    void focus();
    /// The person changed its value (JSON): what it binds takes it.
    void change(std::string_view value);
    /// It tells `tell` with its values (a JSON array).
    void tell(std::string_view tell, std::string_view values);

private:
    ClarityCustomCx* cx_;
};

/// Where the pointer is, with the keys held and the clicks.
struct Pointer {
    double x = 0;
    double y = 0;
    uint32_t modifiers = 0;
    uint32_t clicks = 0;
};

/// A program's own kind of part. Every call comes on the screen's thread.
class CustomPart {
public:
    virtual ~CustomPart() = default;
    /// A property as the window or code sets it: `value`, `enabled` or a declared one (JSON).
    virtual void set(std::string_view name, std::string_view value, Cx& cx) = 0;
    /// What code told it, with its values (a JSON array).
    virtual void hear(std::string_view hear, std::string_view values, Cx& cx) = 0;
    /// Draws itself in its place.
    virtual void draw(Canvas& canvas, double x, double y, double width, double height) = 0;
    virtual void start(Cx&) {}
    virtual bool focusable() { return false; }
    virtual void pointer_pressed(const Pointer&, Cx&) {}
    virtual void pointer_moved(const Pointer&, Cx&) {}
    virtual void pointer_released(const Pointer&, Cx&) {}
    virtual void click(Cx&) {}
    virtual bool wheel(double, double, uint32_t, Cx&) { return false; }
    virtual bool key(std::string_view, uint32_t, Cx&) { return false; }
    virtual void text(std::string_view, Cx&) {}
    virtual void focus_changed(bool, Cx&) {}
    virtual void wake(Cx&) {}
    virtual void stop() {}
    virtual std::optional<std::string> tooltip_at(double, double) { return std::nullopt; }
};

/// Adds a kind of custom part by its declaration's name; `make` makes each of its parts.
void register_part(std::string_view kind, std::function<std::unique_ptr<CustomPart>()> make);

}  // namespace clarity::ui
