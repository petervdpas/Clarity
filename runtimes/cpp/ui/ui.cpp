#include "ui/ui.hpp"

#include <cstdio>
#include <utility>

namespace clarity::ui {

namespace {

const char* chars(std::string_view text) { return text.empty() ? "" : text.data(); }

void answer(ClarityAnswer* to, std::string_view text) { clarity_answer_set(to, reinterpret_cast<const uint8_t*>(chars(text)), text.size()); }

/// What a callback threw never crosses into the library: it is written to the standard error.
template <typename F>
void quietly(const char* where, F&& f) {
    try {
        f();
    } catch (const std::exception& e) {
        std::fprintf(stderr, "%s: %s\n", where, e.what());
    } catch (...) {
        std::fprintf(stderr, "%s: it stopped\n", where);
    }
}

}  // namespace

void check(ClarityStatus status) {
    if (status == CLARITY_OK) return;
    std::string message = take(clarity_last_error());
    throw Error(status, message.empty() ? "Clarity.UI failed" : message);
}

std::string take(char* text) {
    if (!text) return {};
    std::string out(text);
    clarity_string_free(text);
    return out;
}

std::string version() { return clarity_ui_version(); }

bool fits() { return clarity_ui_interface() == CLARITY_UI_INTERFACE; }

// -- Files --------------------------------------------------------------------

Files::Files() { check(clarity_files_new(&files_)); }

Files::Files(Files&& other) noexcept : files_(std::exchange(other.files_, nullptr)) {}

Files& Files::operator=(Files&& other) noexcept {
    std::swap(files_, other.files_);
    return *this;
}

Files::~Files() { clarity_files_free(files_); }

Files& Files::window(std::string_view name, std::string_view markup) {
    check(clarity_files_window(files_, chars(name), name.size(), chars(markup), markup.size()));
    return *this;
}

Files& Files::app(std::string_view name, std::string_view markup) {
    check(clarity_files_app(files_, chars(name), name.size(), chars(markup), markup.size()));
    return *this;
}

Files& Files::theme(std::string_view name, std::string_view json) {
    check(clarity_files_theme(files_, chars(name), name.size(), chars(json), json.size()));
    return *this;
}

Files& Files::model(std::string_view json) {
    check(clarity_files_model(files_, chars(json), json.size()));
    return *this;
}

Files& Files::resource(std::string_view name, std::string_view bytes) {
    check(clarity_files_resource(files_, chars(name), name.size(), reinterpret_cast<const uint8_t*>(chars(bytes)), bytes.size()));
    return *this;
}

Files& Files::title(std::string_view title) {
    check(clarity_files_title(files_, chars(title), title.size()));
    return *this;
}

Files& Files::size(double width, double height) {
    check(clarity_files_size(files_, width, height));
    return *this;
}

// -- The program --------------------------------------------------------------

namespace {

void program_call(void* user, const char* command, size_t command_len, const char* args, size_t args_len, ClarityAnswer* to) {
    quietly("program", [&] {
        auto answered = static_cast<Program*>(user)->call({command, command_len}, {args, args_len});
        if (answered) answer(to, *answered);
    });
}

}  // namespace

ProgramHandle::ProgramHandle(Program& program) {
    ClarityProgramCallbacks callbacks{&program, program_call, nullptr};
    check(clarity_program_new(&callbacks, &program_));
}

ProgramHandle::~ProgramHandle() { clarity_program_free(program_); }

int run_desktop(Program& program, const Files& files) {
    ProgramHandle handle(program);
    int32_t code = 0;
    check(clarity_desktop_run(handle.handle(), files.handle(), &code));
    return code;
}

void wake_desktop() { clarity_desktop_wake(); }

Driven::Driven(Program& program, const Files& files) : program_(program) { check(clarity_driven_start(program_.handle(), files.handle(), &driven_)); }

Driven::~Driven() { clarity_driven_free(driven_); }

std::string Driven::step(std::string_view step) {
    char* answered = nullptr;
    ClarityStatus status = clarity_driven_step(driven_, chars(step), step.size(), &answered);
    if (answered) return take(answered);
    check(status);
    return {};
}

// -- Custom parts -------------------------------------------------------------

void Canvas::fill(double x, double y, double width, double height, Color color, double radius) { check(clarity_canvas_fill(canvas_, x, y, width, height, color, radius)); }

void Canvas::stroke(double x, double y, double width, double height, Color color, double line_width, double radius) {
    check(clarity_canvas_stroke(canvas_, x, y, width, height, color, line_width, radius));
}

void Canvas::line(double x1, double y1, double x2, double y2, Color color, double line_width) { check(clarity_canvas_line(canvas_, x1, y1, x2, y2, color, line_width)); }

void Canvas::oval(double x, double y, double width, double height, Color color, bool filled, double line_width) {
    check(clarity_canvas_oval(canvas_, x, y, width, height, color, filled ? 1 : 0, line_width));
}

void Canvas::path(std::string_view svg, Color color, bool filled, double line_width) { check(clarity_canvas_path(canvas_, chars(svg), svg.size(), color, filled ? 1 : 0, line_width)); }

void Canvas::text(std::string_view text, double x, double y, double width, double height, Color color, double size, bool bold, int align) {
    check(clarity_canvas_text(canvas_, chars(text), text.size(), x, y, width, height, color, size, bold ? 1 : 0, align));
}

double Canvas::measure(std::string_view text, double size, bool bold) {
    double width = 0;
    check(clarity_canvas_measure(canvas_, chars(text), text.size(), size, bold ? 1 : 0, &width));
    return width;
}

void Canvas::pixels(const uint8_t* rgba, uint32_t width, uint32_t height, size_t stride, double x, double y, double box_width, double box_height) {
    check(clarity_canvas_pixels(canvas_, rgba, width, height, stride, x, y, box_width, box_height));
}

void Canvas::clip(double x, double y, double width, double height) { check(clarity_canvas_clip(canvas_, x, y, width, height)); }

void Canvas::restore() { check(clarity_canvas_restore(canvas_)); }

Color Canvas::look(std::string_view role) {
    Color color = 0;
    check(clarity_canvas_look_color(canvas_, chars(role), role.size(), &color));
    return color;
}

Cx::Bounds Cx::bounds() const {
    Bounds b;
    check(clarity_custom_bounds(cx_, &b.x, &b.y, &b.width, &b.height));
    return b;
}

bool Cx::enabled() const {
    int32_t on = 1;
    check(clarity_custom_enabled(cx_, &on));
    return on != 0;
}

void Cx::invalidate() { check(clarity_custom_invalidate(cx_)); }
void Cx::wake_in(double ms) { check(clarity_custom_wake_in(cx_, ms)); }
void Cx::animate(bool on) { check(clarity_custom_animate(cx_, on ? 1 : 0)); }
void Cx::focus() { check(clarity_custom_focus(cx_)); }
void Cx::change(std::string_view value) { check(clarity_custom_change(cx_, chars(value), value.size())); }
void Cx::tell(std::string_view tell, std::string_view values) { check(clarity_custom_tell(cx_, chars(tell), tell.size(), chars(values), values.size())); }

namespace {

/// A registered kind: how its parts are made. Kept for the process, as the library keeps the table.
struct Kind {
    std::string name;
    std::function<std::unique_ptr<CustomPart>()> make;
};

CustomPart* part_of(void* part) { return static_cast<CustomPart*>(part); }

Pointer pointer_of(const ClarityPointer* at) { return at ? Pointer{at->x, at->y, at->modifiers, at->clicks} : Pointer{}; }

void* make_part(void* user) {
    void* made = nullptr;
    quietly("custom part", [&] { made = static_cast<Kind*>(user)->make().release(); });
    return made;
}

/// Runs a call of a part with its cx, unless the part could not be made.
template <typename F>
void with_part(void* part, ClarityCustomCx* cx, F&& f) {
    if (!part) return;
    Cx context(cx);
    quietly("custom part", [&] { f(*part_of(part), context); });
}

const ClarityCustomCallbacks callbacks_template{
    nullptr,
    make_part,
    [](void* part, const char* name, size_t name_len, const char* value, size_t value_len, ClarityCustomCx* cx) {
        with_part(part, cx, [&](CustomPart& p, Cx& c) { p.set({name, name_len}, {value, value_len}, c); });
    },
    [](void* part, const char* hear, size_t hear_len, const char* values, size_t values_len, ClarityCustomCx* cx) {
        with_part(part, cx, [&](CustomPart& p, Cx& c) { p.hear({hear, hear_len}, {values, values_len}, c); });
    },
    [](void* part, ClarityCanvas* canvas, double x, double y, double width, double height) {
        if (!part) return;
        Canvas drawn(canvas);
        quietly("custom part", [&] { part_of(part)->draw(drawn, x, y, width, height); });
    },
    [](void* part, ClarityCustomCx* cx) { with_part(part, cx, [](CustomPart& p, Cx& c) { p.start(c); }); },
    [](void* part) -> int32_t {
        bool takes = false;
        if (part) quietly("custom part", [&] { takes = part_of(part)->focusable(); });
        return takes ? 1 : 0;
    },
    [](void* part, const ClarityPointer* at, ClarityCustomCx* cx) { with_part(part, cx, [&](CustomPart& p, Cx& c) { p.pointer_pressed(pointer_of(at), c); }); },
    [](void* part, const ClarityPointer* at, ClarityCustomCx* cx) { with_part(part, cx, [&](CustomPart& p, Cx& c) { p.pointer_moved(pointer_of(at), c); }); },
    [](void* part, const ClarityPointer* at, ClarityCustomCx* cx) { with_part(part, cx, [&](CustomPart& p, Cx& c) { p.pointer_released(pointer_of(at), c); }); },
    [](void* part, ClarityCustomCx* cx) { with_part(part, cx, [](CustomPart& p, Cx& c) { p.click(c); }); },
    [](void* part, double dx, double dy, uint32_t modifiers, ClarityCustomCx* cx) -> int32_t {
        bool took = false;
        with_part(part, cx, [&](CustomPart& p, Cx& c) { took = p.wheel(dx, dy, modifiers, c); });
        return took ? 1 : 0;
    },
    [](void* part, const char* name, size_t name_len, uint32_t modifiers, ClarityCustomCx* cx) -> int32_t {
        bool took = false;
        with_part(part, cx, [&](CustomPart& p, Cx& c) { took = p.key({name, name_len}, modifiers, c); });
        return took ? 1 : 0;
    },
    [](void* part, const char* text, size_t text_len, ClarityCustomCx* cx) { with_part(part, cx, [&](CustomPart& p, Cx& c) { p.text({text, text_len}, c); }); },
    [](void* part, int32_t focused, ClarityCustomCx* cx) { with_part(part, cx, [&](CustomPart& p, Cx& c) { p.focus_changed(focused != 0, c); }); },
    [](void* part, ClarityCustomCx* cx) { with_part(part, cx, [](CustomPart& p, Cx& c) { p.wake(c); }); },
    [](void* part) {
        if (part) quietly("custom part", [&] { part_of(part)->stop(); });
    },
    [](void* part, double x, double y, ClarityAnswer* tip) {
        if (!part) return;
        quietly("custom part", [&] {
            if (auto text = part_of(part)->tooltip_at(x, y)) answer(tip, *text);
        });
    },
    [](void* part) { delete part_of(part); },
};

}  // namespace

void register_part(std::string_view kind, std::function<std::unique_ptr<CustomPart>()> make) {
    // The library keeps the table for the process, so the kind is kept too.
    auto* kept = new Kind{std::string(kind), std::move(make)};
    ClarityCustomCallbacks callbacks = callbacks_template;
    callbacks.user = kept;
    check(clarity_custom_register(chars(kind), kind.size(), &callbacks));
}

}  // namespace clarity::ui
