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
    /// The text of key `key`, with its value in the person's language.
    static Text key(const std::string& key);

    /// The slot at `path` gets its value.
    Text with(const std::string& path, Slot value) const;
    /// The text with its slots filled plainly, without a person's format;
    /// a key takes its value in the language the screen picked.
    std::string show() const;

    const std::string& text() const { return template_; }
    /// The key the text is, when it is one: the screen shows its value in the person's language.
    const std::optional<std::string>& key() const { return key_; }
    const std::map<std::string, Slot>& values() const { return values_; }

private:
    std::string template_;
    std::map<std::string, Slot> values_;
    std::optional<std::string> key_;
};

/// Takes every language's values from the model (`project.translations`)
/// and the default language (`project.locale.language`), for texts by key
/// (conformance/translations.json, `code`).
void load_translations(const std::string& model);
/// The language texts show in, as the screen picked it; none for the default.
void set_language(std::optional<std::string> language);
/// The language the screen picked; none for the default.
std::optional<std::string> language();
/// The value of `key` in the language picked, or the default language's
/// when that one leaves it empty; none when no language has the key.
std::optional<std::string> key_value(const std::string& key);
/// The value of the key `key_or_text` names, or the text itself when it
/// names none: what an engine gives.
std::string text_of(const std::string& key_or_text);

/// Text without the spaces it starts and ends with.
std::string trimmed(const std::string& text);
/// Text in lower case, as a search or a sort that ignores case compares it:
/// ASCII, and the capitals of Latin-1.
std::string lower_case(const std::string& text);

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

namespace nlohmann {

/// A date that may be none, null when it is: an item of a list of dates.
template <>
struct adl_serializer<std::optional<clarity::Date>> {
    static void to_json(json& out, const std::optional<clarity::Date>& d) { out = d ? json(*d) : json(nullptr); }
    static void from_json(const json& in, std::optional<clarity::Date>& d) {
        if (in.is_null()) d.reset();
        else d = in.get<clarity::Date>();
    }
};

}  // namespace nlohmann
