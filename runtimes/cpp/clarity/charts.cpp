#include "clarity/charts.hpp"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <optional>

#include "clarity/text.hpp"

namespace clarity {

using nlohmann::json;

namespace {

bool no_value(const json& v) { return v.is_null() || (v.is_string() && v.get<std::string>().empty()); }

std::string text_of_value(const json& v) { return v.is_string() ? v.get<std::string>() : v.dump(); }

/// A value as a number: none when it is empty or no number.
std::optional<double> number_of(const json& v) {
    if (no_value(v)) return std::nullopt;
    if (v.is_number()) return v.get<double>();
    if (v.is_boolean()) return v.get<bool>() ? 1.0 : 0.0;
    std::string t = trimmed(text_of_value(v));
    if (t.empty()) return 0.0;
    char* end = nullptr;
    double n = std::strtod(t.c_str(), &end);
    if (end != t.c_str() + t.size() || std::isnan(n)) return std::nullopt;
    return n;
}

/// To 12 decimals, rounded half up; a whole number as one.
json rounded(double n) {
    double r = std::floor(n * 1e12 + 0.5) / 1e12;
    if (r == std::floor(r) && std::fabs(r) < 9e15) return int64_t(r);
    return r;
}

int64_t place_in(const json& order, const json& v) {
    if (!order.is_array()) return -1;
    auto found = std::find(order.begin(), order.end(), v);
    return found == order.end() ? -1 : int64_t(found - order.begin());
}

}  // namespace

std::vector<ChartGroup> chart_groups(const std::vector<ChartRow>& rows, const std::string& by, const std::string& measure, const json& order) {
    struct Group {
        json key;
        int64_t count = 0;
        double sum = 0;
        int64_t numbers = 0;
    };
    std::vector<Group> groups;
    for (const auto& row : rows) {
        if (no_value(row.by)) continue;
        auto g = std::find_if(groups.begin(), groups.end(), [&](const Group& x) { return x.key == row.by; });
        if (g == groups.end()) g = groups.insert(groups.end(), Group{row.by});
        g->count += 1;
        if (auto n = number_of(row.of)) {
            g->sum += *n;
            g->numbers += 1;
        }
    }
    auto before = [&](const Group& ga, const Group& gb) {
        const json& a = ga.key;
        const json& b = gb.key;
        if (by == "number") return number_of(a).value_or(NAN) < number_of(b).value_or(NAN);
        if (by == "enum") return place_in(order, a) < place_in(order, b);
        if (by == "bool") return int(a == json(true)) < int(b == json(true));
        std::string sa = text_of_value(a), sb = text_of_value(b);
        if (by == "date") return sa < sb;
        std::string la = lower_case(sa), lb = lower_case(sb);
        if (la != lb) return la < lb;
        return sa < sb;
    };
    std::stable_sort(groups.begin(), groups.end(), before);
    std::vector<ChartGroup> out;
    for (const auto& g : groups) {
        double value = measure == "count" ? double(g.count) : measure == "sum" ? g.sum : g.numbers > 0 ? g.sum / double(g.numbers) : 0;
        out.push_back({g.key, rounded(value)});
    }
    return out;
}

}  // namespace clarity
