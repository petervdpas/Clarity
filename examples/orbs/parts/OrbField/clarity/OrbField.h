// Custom part OrbField (DESIGN 5.19 and 7.9): the class its library implements, a
// control of Clarity.UI. Written by Clarity from the part's declaration, and
// written again when the declaration changes; the files beside this folder
// are yours.
//
// Every call comes on the screen's thread. The part draws on the canvas
// Clarity.UI gives, takes the pointer and keys (clarity::ui::CustomPart in
// ui/ui.hpp) and tells its window through the cx of the call. An integer is
// int64_t and a bool is bool; every other value (text, a decimal, money, a
// date, an enum) is text in its plain form.

#pragma once

#include <cstdint>
#include <cstdlib>
#include <memory>
#include <string>
#include <string_view>

#include <nlohmann/json.hpp>

#include "ui/ui.hpp"

class OrbFieldPart : public clarity::ui::CustomPart {
public:
    /// Whether the person can use it, as code sets `enabled`.
    virtual void show_enabled(bool enabled, clarity::ui::Cx& cx) {
        (void)enabled;
        cx.invalidate();
    }
    /// Property speed, as the window or code sets it.
    virtual void show_speed(int64_t speed, clarity::ui::Cx& cx) = 0;
    /// What code tells it: OrbField.restart.
    virtual void on_restart(clarity::ui::Cx& cx) = 0;

    void set(std::string_view name, std::string_view value, clarity::ui::Cx& cx) final {
        nlohmann::json given = nlohmann::json::parse(value, nullptr, false);
        if (name == "enabled") show_enabled(given.is_boolean() && given.get<bool>(), cx);
        else if (name == "speed") show_speed(integer_of(given), cx);
    }

    void hear(std::string_view hear, std::string_view values, clarity::ui::Cx& cx) final {
        nlohmann::json given = nlohmann::json::parse(values, nullptr, false);
        if (!given.is_array()) given = nlohmann::json::array();
        (void)given;
        if (hear == "restart") on_restart(cx);
    }

protected:
    /// Tells caught: the handler wired to it runs with these values.
    void caught(clarity::ui::Cx& cx, int64_t points) { cx.tell("caught", nlohmann::json::array({points}).dump()); }
    /// Tells finished: the handler wired to it runs with these values.
    void finished(clarity::ui::Cx& cx, int64_t points) { cx.tell("finished", nlohmann::json::array({points}).dump()); }

private:
    static int64_t integer_of(const nlohmann::json& v) {
        if (v.is_number()) return v.get<int64_t>();
        if (v.is_string()) return std::strtoll(v.get<std::string>().c_str(), nullptr, 10);
        return 0;
    }
    static std::string text_of(const nlohmann::json& v) {
        if (v.is_string()) return v.get<std::string>();
        if (v.is_null() || v.is_discarded()) return {};
        return v.dump();
    }
    static nlohmann::json at(const nlohmann::json& values, size_t i) { return i < values.size() ? values[i] : nlohmann::json(); }
};

/// Makes a new OrbField; the part's library implements it.
std::unique_ptr<OrbFieldPart> make_OrbField();
