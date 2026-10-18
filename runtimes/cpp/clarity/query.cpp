#include "clarity/query.hpp"

#include <algorithm>
#include <cctype>

namespace clarity {

struct Cond::Node {
    enum class Kind { Compare, Contains, OneOf, IsTrue, IsNone, Through, And, Or, Not } kind;
    std::string column;
    Op op = Op::Eq;
    std::vector<SqlValue> values;
    bool numeric = false;
    std::string text;
    Hop hop;
    std::vector<Cond> inner;
};

namespace {

std::string op_sql(Op op) {
    switch (op) {
    case Op::Eq: return "=";
    case Op::NotEq: return "!=";
    case Op::Lt: return "<";
    case Op::LtEq: return "<=";
    case Op::Gt: return ">";
    default: return ">=";
    }
}

std::string column_sql(Dialect d, const std::string& column, bool numeric) {
    return numeric ? number(d, quote(d, column)) : quote(d, column);
}

}  // namespace

Cond Cond::compare(const std::string& column, Op op, SqlValue value) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::Compare, column, op, {std::move(value)}, false, "", {}, {}}));
}

Cond Cond::number(const std::string& column, Op op, SqlValue value) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::Compare, column, op, {std::move(value)}, true, "", {}, {}}));
}

Cond Cond::contains(const std::string& column, const std::string& text) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::Contains, column, Op::Eq, {}, false, text, {}, {}}));
}

Cond Cond::one_of(const std::string& column, std::vector<SqlValue> values) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::OneOf, column, Op::Eq, std::move(values), false, "", {}, {}}));
}

Cond Cond::one_of_numbers(const std::string& column, std::vector<SqlValue> values) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::OneOf, column, Op::Eq, std::move(values), true, "", {}, {}}));
}

Cond Cond::is_true(const std::string& column) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::IsTrue, column, Op::Eq, {}, false, "", {}, {}}));
}

Cond Cond::is_none(const std::string& column) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::IsNone, column, Op::Eq, {}, false, "", {}, {}}));
}

Cond Cond::through(const std::string& column, const std::string& table, const std::string& key, Cond cond) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::Through, "", Op::Eq, {}, false, "", Hop{column, table, key}, {std::move(cond)}}));
}

Cond operator&&(Cond a, Cond b) {
    return Cond(std::make_shared<Cond::Node>(Cond::Node{Cond::Node::Kind::And, "", Op::Eq, {}, false, "", {}, {std::move(a), std::move(b)}}));
}

Cond operator||(Cond a, Cond b) {
    return Cond(std::make_shared<Cond::Node>(Cond::Node{Cond::Node::Kind::Or, "", Op::Eq, {}, false, "", {}, {std::move(a), std::move(b)}}));
}

Cond operator!(Cond a) {
    return Cond(std::make_shared<Cond::Node>(Cond::Node{Cond::Node::Kind::Not, "", Op::Eq, {}, false, "", {}, {std::move(a)}}));
}

std::string Cond::sql(Dialect d, std::vector<SqlValue>& params) const {
    const Node& n = *node_;
    auto param = [&](SqlValue value, bool numeric) {
        params.push_back(std::move(value));
        std::string m = mark(d, int(params.size()));
        return numeric ? clarity::number(d, m) : m;
    };
    switch (n.kind) {
    case Node::Kind::Compare: {
        std::string column = column_sql(d, n.column, n.numeric);
        return column + " " + op_sql(n.op) + " " + param(n.values[0], n.numeric);
    }
    case Node::Kind::Contains:
        return clarity::contains(d, n.column, param("%" + escape_like(d, n.text) + "%", false));
    case Node::Kind::OneOf: {
        std::string out = column_sql(d, n.column, n.numeric) + " IN (";
        for (size_t i = 0; i < n.values.size(); ++i) out += (i ? ", " : "") + param(n.values[i], n.numeric);
        return out + ")";
    }
    case Node::Kind::IsTrue: return clarity::is_true(d, n.column);
    case Node::Kind::IsNone: return quote(d, n.column) + " IS NULL";
    // The inner condition names the columns of the referred table, which a
    // subquery's own FROM resolves first, even for a table that refers to itself.
    case Node::Kind::Through:
        return quote(d, n.hop.column) + " IN (SELECT " + quote(d, n.hop.key) + " FROM " + quote(d, n.hop.table) + " WHERE " + n.inner[0].sql(d, params) + ")";
    case Node::Kind::And: {
        std::string left = n.inner[0].sql(d, params);
        return "(" + left + " AND " + n.inner[1].sql(d, params) + ")";
    }
    case Node::Kind::Or: {
        std::string left = n.inner[0].sql(d, params);
        return "(" + left + " OR " + n.inner[1].sql(d, params) + ")";
    }
    default: return "NOT (" + n.inner[0].sql(d, params) + ")";
    }
}

Query Query::filter(Cond cond) const {
    Query q = *this;
    q.filter_ = std::make_shared<Cond>(std::move(cond));
    return q;
}

Query Query::order(const std::string& column, bool descending) const { return order_through({}, column, descending, false); }
Query Query::order_number(const std::string& column, bool descending) const { return order_through({}, column, descending, true); }

Query Query::order_through(std::vector<Hop> through, const std::string& column, bool descending, bool numeric) const {
    Query q = *this;
    q.order_.push_back({std::move(through), column, descending, numeric});
    return q;
}

// Through references, each step is a subquery on its own alias (t1, t2, ...)
// that finds the referred record by the column of the step before.
std::string Query::key_sql(Dialect d, const std::string& table, const Key& key) {
    std::string direction = key.descending ? " DESC" : "";
    if (key.through.empty()) return column_sql(d, key.column, key.numeric) + direction;
    std::string inner;
    std::string outer = quote(d, table);
    std::vector<std::string> aliases;
    for (size_t i = 0; i < key.through.size(); ++i) aliases.push_back(quote(d, "t" + std::to_string(i + 1)));
    std::string column = aliases.back() + "." + quote(d, key.column);
    inner = key.numeric ? number(d, column) : column;
    for (size_t i = key.through.size(); i-- > 0;) {
        const Hop& hop = key.through[i];
        std::string before = i == 0 ? outer : aliases[i - 1];
        inner = "(SELECT " + inner + " FROM " + quote(d, hop.table) + " " + aliases[i] + " WHERE " + aliases[i] + "." + quote(d, hop.key) + " = " + before + "." + quote(d, hop.column) + ")";
    }
    return inner + direction;
}

std::pair<std::string, std::vector<SqlValue>> Query::sql(Dialect d, const std::string& table, const std::string& key, const std::string& columns) const {
    std::vector<SqlValue> params;
    std::string sql = "SELECT " + columns + " FROM " + quote(d, table);
    if (filter_) sql += " WHERE " + filter_->sql(d, params);
    sql += " ORDER BY ";
    for (const Key& k : order_) sql += key_sql(d, table, k) + ", ";
    sql += quote(d, key);
    return {sql, params};
}

bool text_contains(const std::string& text, const std::string& part) {
    auto lower = [](std::string s) {
        std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) { return char(std::tolower(c)); });
        return s;
    };
    return lower(text).find(lower(part)) != std::string::npos;
}

}  // namespace clarity
