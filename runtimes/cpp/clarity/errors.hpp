// What goes wrong, as exceptions: each kind the screen and code tell apart.
// The other runtimes have the same kinds.

#pragma once

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>

#include "clarity/text.hpp"

namespace clarity {

/// Anything that goes wrong in a generated application.
class Error : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

/// The database or the disk failed.
class StorageError : public Error {
public:
    explicit StorageError(const std::string& what) : Error("storage: " + what), problem_(what) {}
    /// What went wrong, without the kind before it.
    const std::string& problem() const { return problem_; }

private:
    std::string problem_;
};

/// A database connects with a secret setting (manual/data.settings.md) the person has not kept yet.
class NotKeptError : public StorageError {
public:
    explicit NotKeptError(const std::string& setting) : StorageError("the secret setting " + setting + " is not kept yet: the settings name the connection") {}
};

/// A database or file store at `env:NAME` while that environment variable is
/// unset or empty (conformance/data-folder.json): said in Clarity's own
/// words, naming the variable.
class NotSetError : public Error {
public:
    explicit NotSetError(const std::string& variable) : Error("the environment variable " + variable + " is not set"), variable(variable) {}
    std::string variable;
    Text words() const { return Text::keyed("variableNotSet", "The environment variable {variable} must be set").with("variable", slot_text(variable)); }
};

/// Why the databases did not open, as the person reads it (manual/data.settings.md,
/// conformance/unconnected.json): in Clarity's own words, which the project
/// translates, with this runtime's message in the slot.
inline Text unconnected_reason(const std::exception& e) {
    if (dynamic_cast<const NotKeptError*>(&e)) return Text::key("unconnectedNotKept");
    // A database at an unset environment variable says which (conformance/data-folder.json).
    if (auto unset = dynamic_cast<const NotSetError*>(&e)) return unset->words();
    if (auto storage = dynamic_cast<const StorageError*>(&e)) return Text::key("unconnectedFailed").with("problem", slot_text(storage->problem()));
    return Text::key("unconnectedFailed").with("problem", slot_text(e.what()));
}

/// A database whose migration steps are not the ones the program was made with
/// (docs/ledger/migrations.md): its message is the English line the program stops with.
class StepsError : public Error {
public:
    using Error::Error;
};

/// A value does not fit: a wrong date, a reference to nothing, a file that is no picture.
class InvalidError : public Error {
public:
    explicit InvalidError(const std::string& what) : Error("invalid value: " + what) {}
};

/// A record that is not stored, or a reference that is empty.
class NotFoundError : public Error {
public:
    NotFoundError(std::string table, std::optional<int64_t> id)
        : Error(id ? "there is no " + table + " " + std::to_string(*id) : "the " + table + " is not set"), table(std::move(table)), id(id) {}
    std::string table;
    std::optional<int64_t> id;
};

/// A record that others still refer to cannot be deleted.
class InUseError : public Error {
public:
    InUseError(std::string table, int64_t id)
        : Error("the " + table + " " + std::to_string(id) + " is still in use"), table(std::move(table)), id(id) {}
    std::string table;
    int64_t id;
};

/// `refuse` or a failed contract: the text the person reads, with its slots.
class RefusedError : public Error {
public:
    explicit RefusedError(Text text) : Error(text.show()), text(std::move(text)) {}
    Text text;
};

/// `refuse text`.
[[noreturn]] inline void refuse(Text text) { throw RefusedError(std::move(text)); }

/// A value in code that does not fit where it is used: a position outside
/// its list, a highest number below 1 (conformance/lists.json); with
/// Clarity's own words for it, which a window shows, when it has them.
class ValueError : public Error {
public:
    using Error::Error;
    ValueError(const std::string& what, Text words) : Error(what), words(std::move(words)) {}
    std::optional<Text> words;
};

/// A number that does not fit (conformance/overflow.json).
inline ValueError too_large() { return ValueError("a number too large", Text::keyed("numberTooLarge", "The number is too large")); }

/// Code listed or wrote a folder that is no store, with no folder chosen.
inline ValueError no_folder_chosen() { return ValueError("no folder was chosen", Text::keyed("noFolderChosen", "No folder was chosen")); }

/// a / 0, a div 0 or a mod 0 (conformance/decimal-division.json).
inline ValueError division_by_zero() { return ValueError("division by zero", Text::keyed("divisionByZero", "Division by zero")); }

/// What a window shows when code fails with `e`: a refusal its own text; a
/// value or a record that is not there, or still used, in Clarity's own words
/// by a literal key (translated where the project has them). None for any
/// other error, which a window shows by its message, the English of the
/// shared cases.
inline std::optional<Text> said(const std::exception& e) {
    if (auto refused = dynamic_cast<const RefusedError*>(&e)) return refused->text;
    if (auto value = dynamic_cast<const ValueError*>(&e)) return value->words;
    if (auto unset = dynamic_cast<const NotSetError*>(&e)) return unset->words();
    if (auto missing = dynamic_cast<const NotFoundError*>(&e); missing && missing->id)
        return Text::keyed("recordNotThere", "There is no {table} with id {id}").with("table", slot_text(missing->table)).with("id", slot_integer(*missing->id));
    if (auto used = dynamic_cast<const InUseError*>(&e)) return Text::keyed("recordStillUsed", "This {table} is still used by other records").with("table", slot_text(used->table));
    return std::nullopt;
}

/// A statement broke a reference between records; the store turns it into InvalidError or InUseError.
class ReferenceFailure : public Error {
public:
    using Error::Error;
};

}  // namespace clarity
