// The shown rows of a window's tables, lists, trees and charts (DESIGN 1.7):
// the window's state carries a view per such part, by its name, saying what
// it shows of which list and how. The runtime fills in what shows after
// every reply: a table's `shown` places and `pages`, a tree's `shown` rows,
// a chart's `groups`. The person's sort, page and tree rows change the view.

#pragma once

#include <map>
#include <string>

#include <nlohmann/json.hpp>

#include "clarity/ui.hpp"

namespace clarity {

/// Fills each view of `state` from the window's data. A chosen row turns its
/// table to the row's page and opens the tree rows above it. For the reply
/// to a view_event, `sent` is the chosen rows the screen sent, and only a
/// row chosen since counts, so the person's own page and closed rows stay.
void fill_views(WindowState& state, const std::map<std::string, int>* sent = nullptr);

/// The person's action on the view of `part`: `{"sort": field}` (the same
/// field again sorts the other way), `{"page": n}`, `{"open": id}` or
/// `{"close": id}`. An empty one changes nothing and is no error; any other
/// throws InvalidError when the part has no view.
void view_action(WindowState& state, const std::string& part, const nlohmann::json& action);

}  // namespace clarity
