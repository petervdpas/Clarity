#include "clarity/dropdown_options.hpp"

#include <algorithm>
#include <set>
#include <sstream>

#include "clarity/text.hpp"

namespace clarity {

namespace {

/// A field's value as a search reads it: text as it is, nothing for null.
std::string shown_text(const nlohmann::json& v) {
    if (v.is_null()) return "";
    return v.is_string() ? v.get<std::string>() : v.dump();
}

}  // namespace

std::vector<std::string> suggestions(const std::vector<std::string>& values, const std::string& typed) {
    std::string wanted = lower_case(trimmed(typed));
    std::vector<std::string> once;
    std::set<std::string> seen;
    for (const auto& v : values)
        if (!v.empty() && seen.insert(v).second && lower_case(v).find(wanted) != std::string::npos) once.push_back(v);
    std::stable_sort(once.begin(), once.end(), [](const std::string& a, const std::string& b) {
        std::string la = lower_case(a), lb = lower_case(b);
        return la != lb ? la < lb : a < b;
    });
    return once;
}

std::vector<RefOption> searched(const std::vector<RefOption>& options, const std::string& typed, const std::vector<std::string>& show) {
    std::vector<std::string> words;
    std::istringstream in(lower_case(typed));
    for (std::string word; in >> word;) words.push_back(word);
    if (words.empty()) return options;
    std::vector<RefOption> found;
    for (const auto& o : options) {
        std::vector<std::string> texts{lower_case(o.text)};
        if (o.fields.is_object()) {
            if (show.empty()) {
                for (const auto& v : o.fields) texts.push_back(lower_case(shown_text(v)));
            } else {
                for (const auto& name : show)
                    if (o.fields.contains(name)) texts.push_back(lower_case(shown_text(o.fields[name])));
            }
        }
        bool all = std::all_of(words.begin(), words.end(), [&](const std::string& w) {
            return std::any_of(texts.begin(), texts.end(), [&](const std::string& t) { return t.find(w) != std::string::npos; });
        });
        if (all) found.push_back(o);
    }
    return found;
}

}  // namespace clarity
