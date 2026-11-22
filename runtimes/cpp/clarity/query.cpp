#include "clarity/query.hpp"

#include <algorithm>
#include <cctype>

#include <nlohmann/json.hpp>

#include "clarity/text.hpp"

namespace clarity {

struct Cond::Node {
    enum class Kind { Compare, Contains, Holds, Matches, OneOf, IsTrue, IsNone, Through, And, Or, Not } kind;
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

// An item of a list as `holds` looks for it: SQLite compares the item
// itself, PostgreSQL a list of it as JSON, MySQL the item as JSON, and SQL
// Server its text. A date or a decimal is its text, as a list keeps it.
SqlValue list_item(Dialect d, const SqlValue& item) {
    if (auto n = std::get_if<int64_t>(&item)) {
        if (d == Dialect::Sqlite) return *n;
        return d == Dialect::Postgres ? "[" + std::to_string(*n) + "]" : std::to_string(*n);
    }
    std::string text;
    if (auto s = std::get_if<std::string>(&item)) text = *s;
    else if (auto dec = std::get_if<Decimal>(&item)) text = dec->str();
    else if (auto day = std::get_if<Date>(&item)) text = day->str();
    if (d == Dialect::Sqlite || d == Dialect::Mssql) return text;
    std::string json = nlohmann::json(text).dump();
    return d == Dialect::Postgres ? "[" + json + "]" : json;
}

// A word of a search is a run of ASCII letters and digits and of any
// characters beyond ASCII (UTF-8 bytes from 0x80).
bool word_byte(unsigned char b) { return b >= 0x80 || std::isalnum(b); }

std::vector<std::string> words_of(const std::string& text) {
    std::vector<std::string> out;
    std::string word;
    for (unsigned char b : text) {
        if (word_byte(b)) {
            word += char(b);
        } else if (!word.empty()) {
            out.push_back(word);
            word.clear();
        }
    }
    if (!word.empty()) out.push_back(word);
    return out;
}

// The characters of UTF-8 text.
size_t characters(const std::string& text) {
    return size_t(std::count_if(text.begin(), text.end(), [](char c) { return (static_cast<unsigned char>(c) & 0xC0) != 0x80; }));
}

// The shortest word MySQL's full-text index keeps (innodb_ft_min_token_size).
constexpr size_t mysql_shortest = 3;

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

Cond Cond::holds(const std::string& column, SqlValue item) {
    return Cond(std::make_shared<Node>(Node{Node::Kind::Holds, column, Op::Eq, {std::move(item)}, false, "", {}, {}}));
}

Cond Cond::matches(const std::string& table, const std::string& key, const std::vector<std::string>& columns, const std::string& text) {
    // The columns ride in the values, the table and key in the hop.
    std::vector<SqlValue> names(columns.begin(), columns.end());
    return Cond(std::make_shared<Node>(Node{Node::Kind::Matches, "", Op::Eq, std::move(names), false, text, Hop{"", table, key}, {}}));
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
    case Node::Kind::Holds:
        return clarity::holds(d, n.column, param(list_item(d, n.values[0]), false));
    case Node::Kind::Matches: {
        auto words = words_of(n.text);
        if (words.empty()) return "1 = 1";
        std::string columns;
        for (size_t i = 0; i < n.values.size(); ++i) {
            std::string column = quote(d, std::get<std::string>(n.values[i]));
            if (i) columns += d == Dialect::Postgres ? " || ' ' || " : d == Dialect::Mssql ? " + N' ' + " : ", ";
            columns += d == Dialect::Postgres ? "coalesce(" + column + ", '')" : d == Dialect::Mssql ? "ISNULL(" + column + ", N'')" : column;
        }
        std::string query;
        std::vector<std::string> parts;
        switch (d) {
        case Dialect::Sqlite: {
            for (size_t i = 0; i < words.size(); ++i) query += (i ? " AND " : "") + ("\"" + words[i] + "\"*");
            std::string index = quote(d, n.hop.table + "__search");
            return quote(d, n.hop.key) + " IN (SELECT rowid FROM " + index + " WHERE " + index + " MATCH " + param(query, false) + ")";
        }
        case Dialect::Postgres:
            for (size_t i = 0; i < words.size(); ++i) query += (i ? " & " : "") + words[i] + ":*";
            return "to_tsvector('simple', " + columns + ") @@ to_tsquery('simple', " + param(query, false) + ")";
        case Dialect::Mysql:
            // The index leaves out words shorter than 3 characters.
            for (const auto& w : words)
                if (characters(w) >= mysql_shortest) query += (query.empty() ? "+" : " +") + w + "*";
            if (!query.empty()) parts.push_back("MATCH (" + columns + ") AGAINST (" + param(query, false) + " IN BOOLEAN MODE)");
            for (const auto& w : words)
                if (characters(w) < mysql_shortest) parts.push_back("CONCAT_WS(' ', " + columns + ") REGEXP " + param("(^|[^[:alnum:]])" + w, false));
            break;
        default:
            for (const auto& w : words) parts.push_back("(N' ' + " + columns + ") LIKE " + param("%[^0-9A-Za-z]" + w + "%", false));
            break;
        }
        std::string out;
        for (size_t i = 0; i < parts.size(); ++i) out += (i ? " AND " : "") + parts[i];
        return parts.size() > 1 ? "(" + out + ")" : out;
    }
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
    if (key.through.empty()) return order_key(d, column_sql(d, key.column, key.numeric), key.descending);
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
    return order_key(d, inner, key.descending);
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

namespace {

/// A letter's simple lower-case form, as every stack takes it for `contains`
/// (conformance/contains.json): Latin with its accents (Latin-1 and Latin
/// Extended-A), Greek and Cyrillic; any other character as it is.
uint32_t lower_of(uint32_t c) {
    if (c >= 'A' && c <= 'Z') return c + 32;
    if (c >= 0xC0 && c <= 0xDE && c != 0xD7) return c + 32;
    if ((c >= 0x100 && c <= 0x137) || (c >= 0x14A && c <= 0x177)) return c % 2 == 0 ? c + 1 : c;
    if ((c >= 0x139 && c <= 0x148) || (c >= 0x179 && c <= 0x17E)) return c % 2 == 1 ? c + 1 : c;
    if (c == 0x178) return 0xFF;
    if (c >= 0x391 && c <= 0x3A9 && c != 0x3A2) return c + 32;
    if (c == 0x386) return 0x3AC;
    if (c >= 0x388 && c <= 0x38A) return c + 37;
    if (c == 0x38C) return 0x3CC;
    if (c == 0x38E || c == 0x38F) return c + 63;
    if (c >= 0x410 && c <= 0x42F) return c + 32;
    if (c >= 0x400 && c <= 0x40F) return c + 80;
    return c;
}

/// `text` with every letter in its lower-case form (lower_of), as UTF-8.
std::string lowered(const std::string& text) {
    std::string out;
    out.reserve(text.size());
    for (size_t i = 0; i < text.size();) {
        uint32_t c = (unsigned char)text[i];
        size_t len = c < 0x80 ? 1 : (c >> 5) == 0x6 ? 2 : (c >> 4) == 0xE ? 3 : (c >> 3) == 0x1E ? 4 : 1;
        if (len > 1 && i + len <= text.size()) {
            c &= len == 2 ? 0x1F : len == 3 ? 0x0F : 0x07;
            for (size_t k = 1; k < len; ++k) c = (c << 6) | ((unsigned char)text[i + k] & 0x3F);
        } else {
            len = 1;
        }
        c = lower_of(c);
        if (c < 0x80) {
            out += char(c);
        } else if (c < 0x800) {
            out += char(0xC0 | (c >> 6));
            out += char(0x80 | (c & 0x3F));
        } else if (c < 0x10000) {
            out += char(0xE0 | (c >> 12));
            out += char(0x80 | ((c >> 6) & 0x3F));
            out += char(0x80 | (c & 0x3F));
        } else {
            out += char(0xF0 | (c >> 18));
            out += char(0x80 | ((c >> 12) & 0x3F));
            out += char(0x80 | ((c >> 6) & 0x3F));
            out += char(0x80 | (c & 0x3F));
        }
        i += len;
    }
    return out;
}

}  // namespace

bool text_contains(const std::string& text, const std::string& part) {
    return lowered(text).find(lowered(part)) != std::string::npos;
}

}  // namespace clarity
