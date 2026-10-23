// Queries: which records of an entity to read, and in which order.
// Generated code builds a Query from Clarity's find; this file turns it into
// SQL with parameters, in the dialect of the database the records are in.

#pragma once

#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "clarity/dialect.hpp"
#include "clarity/value.hpp"

namespace clarity {

/// A comparison between a column and a value.
enum class Op { Eq, NotEq, Lt, LtEq, Gt, GtEq };

/// A step through a reference: the column that holds it, and the table and
/// key column of the records it refers to.
struct Hop {
    std::string column;
    std::string table;
    std::string key;
};

/// A condition on the columns of one table. `a && b`, `a || b` and `!a`
/// combine conditions.
class Cond {
public:
    struct Node;

    /// `column op value`.
    static Cond compare(const std::string& column, Op op, SqlValue value);
    /// A comparison of a decimal or money column, as numbers.
    static Cond number(const std::string& column, Op op, SqlValue value);
    /// The column's text contains `text`, ignoring case.
    static Cond contains(const std::string& column, const std::string& text);
    /// Whether the JSON list kept in the column holds `item`, exactly: a list
    /// field's `a.tags contains "live"`.
    static Cond holds(const std::string& column, SqlValue item);
    /// Full-text search, `a matches x`: the record of `table` holds every
    /// word of `text`, each as the start of a word, in the `columns` marked
    /// `search`, through the table's full-text index.
    static Cond matches(const std::string& table, const std::string& key, const std::vector<std::string>& columns, const std::string& text);
    static Cond one_of(const std::string& column, std::vector<SqlValue> values);
    /// `one_of` for a decimal or money column.
    static Cond one_of_numbers(const std::string& column, std::vector<SqlValue> values);
    /// A bool column that is true.
    static Cond is_true(const std::string& column);
    /// A column with no value: an empty date, reference or picture (`none`).
    static Cond is_none(const std::string& column);
    /// `cond` on the record `column` refers to, in `table` by `key`.
    static Cond through(const std::string& column, const std::string& table, const std::string& key, Cond cond);

    friend Cond operator&&(Cond a, Cond b);
    friend Cond operator||(Cond a, Cond b);
    friend Cond operator!(Cond a);

    std::string sql(Dialect d, std::vector<SqlValue>& params) const;

private:
    explicit Cond(std::shared_ptr<const Node> node) : node_(std::move(node)) {}
    std::shared_ptr<const Node> node_;
};

/// The records to read: a condition and an order. Without an order, records
/// come in the order they were stored.
class Query {
public:
    Query filter(Cond cond) const;
    Query order(const std::string& column, bool descending = false) const;
    /// `order` for a decimal or money column.
    Query order_number(const std::string& column, bool descending = false) const;
    /// `order` by a column of the record the references `through` lead to.
    Query order_through(std::vector<Hop> through, const std::string& column, bool descending, bool numeric) const;

    /// The SELECT for `table` in `d`, reading `columns`, and its
    /// parameters. Records that sort the same come in the order of `key`.
    std::pair<std::string, std::vector<SqlValue>> sql(Dialect d, const std::string& table, const std::string& key, const std::string& columns) const;

private:
    struct Key {
        std::vector<Hop> through;
        std::string column;
        bool descending;
        bool numeric;
    };
    std::shared_ptr<Cond> filter_;
    std::vector<Key> order_;
    static std::string key_sql(Dialect d, const std::string& table, const Key& key);
};

/// `text contains part`, ignoring case: Clarity's `contains` outside a query.
bool text_contains(const std::string& text, const std::string& part);

}  // namespace clarity
