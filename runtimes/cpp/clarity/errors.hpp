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
    explicit StorageError(const std::string& what) : Error("storage: " + what) {}
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

/// A statement broke a reference between records; the store turns it into InvalidError or InUseError.
class ReferenceFailure : public Error {
public:
    using Error::Error;
};

}  // namespace clarity
