// Text the screen shows (a message, a refusal, a caption, an error): the
// text as written in Clarity with a typed value per slot, so the screen
// fills each slot in the person's format. On the wire it is a plain string,
// or the text with its slot values, as from the other runtimes.

#pragma once

#include <map>
#include <optional>
#include <string>

#include <nlohmann/json.hpp>

#include "clarity/decimal.hpp"
#include "clarity/values.hpp"

namespace clarity {

/// The value of one slot, with what the screen needs to format it.
struct Slot {
    std::string kind;
    nlohmann::json value;
    std::optional<int> scale;
    /// The named format of the field the value comes from.
    std::optional<std::string> format;
    /// The enum of an enum value, for its label.
    std::optional<std::string> enum_name;

    /// The value as plain text, for places that are not a screen.
    std::string plain() const;
};

Slot slot_text(const std::string& value);
/// A whole number; `format` is the field's named format, or empty.
Slot slot_integer(int64_t value, const std::string& format = "");
/// A decimal, as text so no precision is lost; `scale` is the field's decimals, or -1.
Slot slot_decimal(const Decimal& value, int scale = -1, const std::string& format = "");
Slot slot_money(const Decimal& value, const std::string& format = "");
Slot slot_date(const std::optional<Date>& value, const std::string& format = "");
Slot slot_date_time(const std::optional<DateTime>& value, const std::string& format = "");
/// A duration, in whole seconds.
Slot slot_duration(int64_t seconds);
Slot slot_bool(bool value);
/// An enum value, by its name; the screen shows its label.
Slot slot_enum(const std::string& value, const std::string& enum_name);

/// Text to show, with values in slots: Saved {customer.name}.
class Text {
public:
    Text() = default;
    /// Text as written in Clarity, before its slots get values.
    explicit Text(std::string text) : template_(std::move(text)) {}
    /// Text shown as it is: a brace in it is no slot.
    static Text plain(const std::string& text);

    /// The slot at `path` gets its value.
    Text with(const std::string& path, Slot value) const;
    /// The text with its slots filled plainly, without a person's format.
    std::string show() const;
    /// The same text in the language the screen picked, its values kept.
    Text translated() const;

    const std::string& text() const { return template_; }
    const std::map<std::string, Slot>& values() const { return values_; }

private:
    std::string template_;
    std::map<std::string, Slot> values_;
};

/// Takes the translations from the model (`project.translations`), for the
/// text code makes (conformance/translations.json, `code`).
void load_translations(const std::string& model);
/// The language texts show in, as the screen picked it; none for the texts as written.
void set_language(std::optional<std::string> language);
/// The language the screen picked; none for the texts as written.
std::optional<std::string> language();
/// `text` in the language picked, or as written when it has no translation there.
std::string translate(const std::string& text);

/// Decimals travel as text, so no precision is lost; dates as `2026-10-17`.
void to_json(nlohmann::json& out, const Decimal& value);
void from_json(const nlohmann::json& in, Decimal& value);
void to_json(nlohmann::json& out, const Date& value);
void from_json(const nlohmann::json& in, Date& value);
void to_json(nlohmann::json& out, const DateTime& value);
void from_json(const nlohmann::json& in, DateTime& value);

void to_json(nlohmann::json& out, const Slot& slot);
void from_json(const nlohmann::json& in, Slot& slot);
/// A plain string, or the text with its slot values.
void to_json(nlohmann::json& out, const Text& text);
void from_json(const nlohmann::json& in, Text& text);

}  // namespace clarity
