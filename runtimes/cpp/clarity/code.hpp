// What generated code calls beside records and the database: contracts,
// values shown as text, `x in [a, b]`, the files of a store filtered and
// sorted, and a temporary given where a record is changed in place.

#pragma once

#include <algorithm>
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
/// `from to to` in code: the whole numbers from `from` to `to`, both counted;
/// none when `to` is below `from` (conformance/lists.json, `ranges`).
inline std::vector<int64_t> range(int64_t from, int64_t to) {
    std::vector<int64_t> list;
    for (int64_t i = from; i <= to; ++i) list.push_back(i);
    return list;
}

/// `join(texts, separator)` in code: the texts in order with the separator
/// between each two (conformance/lists.json, `joins`).
inline std::string joined(const std::vector<std::string>& texts, const std::string& separator) {
    std::string out;
    for (size_t i = 0; i < texts.size(); ++i) {
        if (i) out += separator;
        out += texts[i];
    }
    return out;
}

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

/// `list contains item` in code: whether a list holds an item, a whole
/// number in a list of decimals by its value.
template <typename T, typename U>
bool holds(const std::vector<T>& list, const U& item) {
    return std::find(list.begin(), list.end(), T(item)) != list.end();
}

}  // namespace clarity
