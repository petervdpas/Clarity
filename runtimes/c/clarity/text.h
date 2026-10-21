// Text the screen shows (a message, a refusal, a caption, an error): the
// text as written in Clarity with a typed value per slot, so the screen
// fills each slot in the person's format. On the wire it is a plain string,
// or the text with its slot values, as from the other runtimes.

#pragma once

#include "cjson/cJSON.h"
#include "clarity/decimal.h"
#include "clarity/values.h"

/// The value of one slot, with what the screen needs to format it.
typedef struct {
    /// text, integer, decimal, money, date, dateTime, duration, bool or enum.
    const char* kind;
    cJSON* value;
    /// The field's decimals, or -1.
    int scale;
    /// The named format of the field the value comes from, or NULL.
    const char* format;
    /// The enum of an enum value, for its label, or NULL.
    const char* enum_name;
} ClaritySlot;

ClaritySlot clarity_slot_text(const char* value);
/// A whole number; `format` is the field's named format, or NULL.
ClaritySlot clarity_slot_integer(int64_t value, const char* format);
/// A decimal, as text so no precision is lost; `scale` is the field's decimals, or -1.
ClaritySlot clarity_slot_decimal(ClarityDecimal value, int scale, const char* format);
ClaritySlot clarity_slot_money(ClarityDecimal value, const char* format);
ClaritySlot clarity_slot_date(ClarityDate value, const char* format);
ClaritySlot clarity_slot_date_time(ClarityDateTime value, const char* format);
/// A duration, in whole seconds.
ClaritySlot clarity_slot_duration(int64_t seconds);
ClaritySlot clarity_slot_bool(bool value);
/// An enum value, by its name; the screen shows its label.
ClaritySlot clarity_slot_enum(const char* value, const char* enum_name);
/// The value as plain text, for places that are not a screen.
const char* clarity_slot_plain(const ClaritySlot* slot);

/// Text to show, with values in slots: Saved {customer.name}.
struct ClarityText {
    const char* text;
    const char** paths;
    ClaritySlot* slots;
    size_t count;
    size_t capacity;
};

/// Text as written in Clarity, before its slots get values.
ClarityText* clarity_text(const char* text);
/// Text shown as it is: a brace in it is no slot.
ClarityText* clarity_plain_text(const char* text);
/// The slot at `path` gets its value; gives `text` back.
ClarityText* clarity_text_with(ClarityText* text, const char* path, ClaritySlot slot);
/// The text with its slots filled plainly, without a person's format.
const char* clarity_text_show(const ClarityText* text);

/// Takes the translations from the model (`project.translations`), for
/// the text code makes (conformance/translations.json, `code`). They last
/// until the next load, outside the scratch arena.
void clarity_load_translations(const char* model);
/// The language texts show in, as the screen picked it; NULL for the texts as written.
void clarity_set_language(const char* language);
/// The language the screen picked; NULL for the texts as written.
const char* clarity_language(void);
/// `text` in the language picked, or as written when it has no translation there.
const char* clarity_translate(const char* text);
/// The same text in the language the screen picked, its values kept; gives `text` back.
ClarityText* clarity_text_translated(ClarityText* text);

/// A plain string, or the text with its slot values.
cJSON* clarity_text_json(const ClarityText* text);
bool clarity_text_from_json(const cJSON* in, ClarityText** out);
cJSON* clarity_slot_json(const ClaritySlot* slot);
bool clarity_slot_from_json(const cJSON* in, ClaritySlot* out);
