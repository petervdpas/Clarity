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

}  // namespace clarity
