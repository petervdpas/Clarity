// What generated code calls beside records and the database: contracts,
// values shown as text, `x in [a, b]`, the files of a store filtered and
// sorted, and a temporary given where a record is changed in place.

#pragma once

#include <functional>
#include <initializer_list>
#include <optional>
#include <string>
#include <vector>

#include "clarity/db.hpp"

namespace clarity {

/// A `require` or `ensure` of a function or action that did not hold.
class ContractError : public Error {
public:
    explicit ContractError(const std::string& what) : Error("condition failed: " + what) {}
};

/// Checks a require or ensure line.
inline void require(bool holds, const std::string& what) {
    if (!holds) throw ContractError(what);
}

/// A value as text, the way `"text" + value` shows it.
inline std::string show(const std::string& value) { return value; }
inline std::string show(int64_t value) { return std::to_string(value); }
inline std::string show(const Decimal& value) { return value.str(); }
inline std::string show(bool value) { return value ? "yes" : "no"; }
inline std::string show(const std::optional<Date>& value) { return value ? value->str() : ""; }
std::string show(const std::optional<DateTime>& value);

/// `value in [a, b, c]`.
template <typename T>
bool one_of(const T& value, std::initializer_list<T> options) {
    for (const auto& option : options)
        if (value == option) return true;
    return false;
}

/// A temporary (a selected row, a record a call gave) where code changes a
/// record in place; it lives to the end of the statement.
template <typename T>
T& lvalue(T&& value) {
    return value;
}

/// The files `keep` holds for, in their order.
std::vector<FileEntry> filter_files(std::vector<FileEntry> files, const std::function<bool(const FileEntry&)>& keep);

/// One key of an order of files: a detail, ascending or not.
struct FileOrder {
    std::string detail;
    bool descending;
};
inline FileOrder asc(const std::string& detail) { return {detail, false}; }
inline FileOrder desc(const std::string& detail) { return {detail, true}; }

/// The files sorted by the keys, the first deciding first; text by its bytes, as on every stack.
std::vector<FileEntry> sort_files(std::vector<FileEntry> files, std::initializer_list<FileOrder> keys);

/// A value in code that does not fit where it is used: a position outside
/// its list, a highest number below 1 (conformance/lists.json).
class ValueError : public Error {
public:
    using Error::Error;
};

/// `xs[i]` in code: item i of a list, counted from 1.
template <typename T>
const T& at(const std::vector<T>& list, int64_t position) {
    if (position < 1 || position > int64_t(list.size()))
        throw ValueError("there is no item " + std::to_string(position) + " in a list of " + std::to_string(list.size()));
    return list[size_t(position - 1)];
}

/// `xs[i] = value` in code: item i of a list to set, counted from 1.
template <typename T>
T& at(std::vector<T>& list, int64_t position) {
    return const_cast<T&>(at(static_cast<const std::vector<T>&>(list), position));
}

/// The numbers `random(max)` gives: splitmix64, the sequence every runtime
/// gives alike from the same seed.
class Splitmix {
public:
    explicit Splitmix(uint64_t seed) : state_(seed) {}
    /// A whole number from 1 to max.
    int64_t next(int64_t max);

private:
    uint64_t state_;
};

/// `random(max)` in an engine written in Clarity, from a sequence seeded
/// once per run of the application.
int64_t random(int64_t max);

}  // namespace clarity
