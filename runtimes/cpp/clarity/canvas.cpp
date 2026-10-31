#include "clarity/canvas.hpp"

#include <algorithm>
#include <charconv>
#include <cmath>
#include <cstdlib>
#include <functional>
#include <map>
#include <vector>

#include "clarity/errors.hpp"
#include "clarity/paths.hpp"
#include "clarity/picture.hpp"

namespace clarity {

using nlohmann::json;

namespace {

const DrawingUnits default_units{1000, 1000};

const std::string svg_start = R"(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 )";

/// A number, or a string that holds one.
std::optional<double> number_of(const json& value) {
    if (value.is_number()) return value.get<double>();
    if (!value.is_string()) return std::nullopt;
    std::string text = value.get<std::string>();
    auto space = [](char c) { return c == ' ' || c == '\t' || c == '\n' || c == '\r'; };
    while (!text.empty() && space(text.back())) text.pop_back();
    size_t start = 0;
    while (start < text.size() && space(text[start])) ++start;
    text = text.substr(start);
    if (text.empty()) return std::nullopt;
    char* end = nullptr;
    double n = std::strtod(text.c_str(), &end);
    if (end != text.c_str() + text.size()) return std::nullopt;
    return n;
}

/// A whole number, rounded half away from zero.
int64_t whole(double n) { return std::llround(n); }

/// A whole number written in text, and nothing else.
std::optional<int64_t> int_of(const std::string& text) {
    int64_t n = 0;
    auto [end, problem] = std::from_chars(text.data(), text.data() + text.size(), n);
    if (problem != std::errc() || end != text.data() + text.size() || text.empty()) return std::nullopt;
    return n;
}

/// `text` with each of `pairs` (from, to) replaced in one pass, the first that fits at each place.
std::string replaced(const std::string& text, const std::vector<std::pair<std::string, std::string>>& pairs) {
    std::string out;
    for (size_t i = 0; i < text.size();) {
        bool found = false;
        for (const auto& [from, to] : pairs) {
            if (text.compare(i, from.size(), from) == 0) {
                out += to;
                i += from.size();
                found = true;
                break;
            }
        }
        if (!found) out += text[i++];
    }
    return out;
}

std::string escaped(const std::string& text) { return replaced(text, {{"&", "&amp;"}, {"<", "&lt;"}, {">", "&gt;"}, {"\"", "&quot;"}}); }

std::string unescaped(const std::string& text) { return replaced(text, {{"&lt;", "<"}, {"&gt;", ">"}, {"&quot;", "\""}, {"&amp;", "&"}}); }

/// The numbers of each shape a vector canvas keeps, and which of them are places.
struct ShapeFields {
    std::vector<std::string> numbers;
    std::vector<std::string> places;
};

const std::map<std::string, ShapeFields>& shape_fields() {
    static const std::map<std::string, ShapeFields> fields{
        {"rect", {{"x", "y", "width", "height"}, {"x", "y"}}},
        {"line", {{"x1", "y1", "x2", "y2", "width"}, {"x1", "y1", "x2", "y2"}}},
        {"ellipse", {{"x", "y", "rx", "ry"}, {"x", "y"}}},
        {"path", {{"width"}, {}}},
        {"text", {{"x", "y", "size"}, {"x", "y"}}},
    };
    return fields;
}

/// What a shape's left-out numbers are (conformance/drawing.json).
double shape_default(const std::string& op, const std::string& name) {
    if ((op == "line" || op == "path") && name == "width") return 1;
    if (op == "text" && name == "size") return 10;
    return 0;
}

std::string text_in(const json& shape, const char* name) {
    return shape.is_object() && shape.contains(name) && shape[name].is_string() ? shape[name].get<std::string>() : "";
}

json field(const json& shape, const std::string& name) { return shape.is_object() && shape.contains(name) ? shape[name] : json(nullptr); }

using Place = std::function<int64_t(double)>;

/// A shape as a vector canvas keeps it: whole numbers, places snapped.
json fit_shape(const json& shape, const Place& place) {
    std::string op = text_in(shape, "op");
    auto fields = shape_fields().find(op);
    if (fields == shape_fields().end()) throw InvalidError("a vector canvas keeps rect, line, ellipse, path and text, not \"" + op + "\"");
    json out = {{"op", op}};
    for (const auto& name : fields->second.numbers) {
        double n = number_of(field(shape, name)).value_or(shape_default(op, name));
        bool is_place = std::find(fields->second.places.begin(), fields->second.places.end(), name) != fields->second.places.end();
        out[name] = is_place ? place(n) : whole(n);
    }
    out["color"] = text_in(shape, "color");
    if (op == "path") {
        json points = json::array();
        json given = field(shape, "points");
        if (given.is_array())
            for (const auto& p : given) points.push_back(place(number_of(p).value_or(0)));
        out["points"] = points;
    }
    if (op == "text") out["text"] = text_in(shape, "text");
    return out;
}

/// Moves shape so its first place is at x, y, the rest with it; false when
/// it is there already or is a line the runtime does not read.
bool moved_shape(json& shape, int64_t x, int64_t y) {
    if (!shape.is_object()) return false;
    std::string op = text_in(shape, "op");
    auto num = [&](const char* name) { return int64_t(number_of(field(shape, name)).value_or(0)); };
    int64_t dx = 0, dy = 0;
    if (op == "rect" || op == "ellipse" || op == "text") {
        dx = x - num("x");
        dy = y - num("y");
        shape["x"] = x;
        shape["y"] = y;
    } else if (op == "line") {
        dx = x - num("x1");
        dy = y - num("y1");
        shape["x1"] = x;
        shape["y1"] = y;
        shape["x2"] = num("x2") + dx;
        shape["y2"] = num("y2") + dy;
    } else if (op == "path") {
        json& points = shape["points"];
        if (!points.is_array() || points.size() < 2) return false;
        dx = x - int64_t(number_of(points[0]).value_or(0));
        dy = y - int64_t(number_of(points[1]).value_or(0));
        for (size_t i = 0; i < points.size(); ++i) points[i] = int64_t(number_of(points[i]).value_or(0)) + (i % 2 == 0 ? dx : dy);
    } else {
        return false;
    }
    return dx != 0 || dy != 0;
}

std::string shape_line(const json& shape) {
    auto n = [&](const char* name) { return std::to_string(whole(number_of(field(shape, name)).value_or(0))); };
    auto text = [&](const char* name) { return escaped(text_in(shape, name)); };
    std::string op = text_in(shape, "op");
    if (op == "rect")
        return "<rect x=\"" + n("x") + "\" y=\"" + n("y") + "\" width=\"" + n("width") + "\" height=\"" + n("height") + "\" fill=\"" + text("color") + "\"/>";
    if (op == "line")
        return "<line x1=\"" + n("x1") + "\" y1=\"" + n("y1") + "\" x2=\"" + n("x2") + "\" y2=\"" + n("y2") + "\" stroke=\"" + text("color") + "\" stroke-width=\"" + n("width") + "\"/>";
    if (op == "ellipse")
        return "<ellipse cx=\"" + n("x") + "\" cy=\"" + n("y") + "\" rx=\"" + n("rx") + "\" ry=\"" + n("ry") + "\" fill=\"" + text("color") + "\"/>";
    if (op == "path") {
        json points = field(shape, "points");
        std::string pairs;
        if (points.is_array())
            for (size_t i = 0; i + 1 < points.size(); i += 2) {
                if (!pairs.empty()) pairs += " ";
                pairs += std::to_string(whole(number_of(points[i]).value_or(0))) + "," + std::to_string(whole(number_of(points[i + 1]).value_or(0)));
            }
        return "<polyline points=\"" + pairs + "\" fill=\"none\" stroke=\"" + text("color") + "\" stroke-width=\"" + n("width") + "\"/>";
    }
    if (op == "text")
        return "<text x=\"" + n("x") + "\" y=\"" + n("y") + "\" fill=\"" + text("color") + "\" font-size=\"" + n("size") + "\">" + text("text") + "</text>";
    return text_in(shape, "text");
}

/// An element on one line: `<name a="1" b="2"/>` or `<name a="1">content</name>`.
struct Element {
    std::string name;
    std::map<std::string, std::string> attrs;
    std::string content;
};

std::optional<Element> svg_element(const std::string& line) {
    if (line.empty() || line[0] != '<') return std::nullopt;
    std::string rest = line.substr(1);
    size_t end = rest.find_first_of(" />");
    if (end == std::string::npos || end == 0) return std::nullopt;
    Element e;
    e.name = rest.substr(0, end);
    rest = rest.substr(end);
    while (true) {
        size_t start = rest.find_first_not_of(' ');
        rest = start == std::string::npos ? "" : rest.substr(start);
        if (rest.starts_with("/>")) {
            if (rest != "/>") return std::nullopt;
            return e;
        }
        if (rest.starts_with(">")) {
            std::string close = "</" + e.name + ">";
            std::string content = rest.substr(1);
            if (!content.ends_with(close)) return std::nullopt;
            e.content = content.substr(0, content.size() - close.size());
            return e;
        }
        size_t eq = rest.find("=\"");
        if (eq == std::string::npos || eq == 0) return std::nullopt;
        size_t close = rest.find('"', eq + 2);
        if (close == std::string::npos) return std::nullopt;
        e.attrs[rest.substr(0, eq)] = unescaped(rest.substr(eq + 2, close - eq - 2));
        rest = rest.substr(close + 1);
    }
}

/// The shape of one line of a drawing: written back, it must be the line
/// again, or the line is kept as it is.
json read_shape(const std::string& line) {
    json kept = {{"op", "svg"}, {"text", line}};
    auto element = svg_element(line);
    if (!element) return kept;
    auto attr = [&](const std::string& name) {
        auto at = element->attrs.find(name);
        return at == element->attrs.end() ? std::string() : at->second;
    };
    json shape = json::object();
    // Each pair is an attribute and the shape's number it is.
    auto pick = [&](std::initializer_list<std::pair<const char*, const char*>> pairs) {
        for (const auto& [from, to] : pairs) {
            auto n = int_of(attr(from));
            if (!n) return false;
            shape[to] = *n;
        }
        return true;
    };
    bool fits = false;
    const std::string& name = element->name;
    if (name == "rect") {
        shape["op"] = "rect";
        shape["color"] = attr("fill");
        fits = pick({{"x", "x"}, {"y", "y"}, {"width", "width"}, {"height", "height"}});
    } else if (name == "line") {
        shape["op"] = "line";
        shape["color"] = attr("stroke");
        fits = pick({{"x1", "x1"}, {"y1", "y1"}, {"x2", "x2"}, {"y2", "y2"}, {"stroke-width", "width"}});
    } else if (name == "ellipse") {
        shape["op"] = "ellipse";
        shape["color"] = attr("fill");
        fits = pick({{"cx", "x"}, {"cy", "y"}, {"rx", "rx"}, {"ry", "ry"}});
    } else if (name == "polyline") {
        shape["op"] = "path";
        shape["color"] = attr("stroke");
        fits = pick({{"stroke-width", "width"}});
        json points = json::array();
        std::string all = attr("points");
        size_t i = 0;
        while (i < all.size()) {
            size_t start = all.find_first_not_of(" \t\n\r", i);
            if (start == std::string::npos) break;
            size_t stop = all.find_first_of(" \t\n\r", start);
            std::string pair = all.substr(start, stop == std::string::npos ? std::string::npos : stop - start);
            i = stop == std::string::npos ? all.size() : stop;
            size_t comma = pair.find(',');
            auto x = comma == std::string::npos ? std::nullopt : int_of(pair.substr(0, comma));
            auto y = comma == std::string::npos ? std::nullopt : int_of(pair.substr(comma + 1));
            if (!x || !y) fits = false;
            points.push_back(x.value_or(0));
            points.push_back(y.value_or(0));
        }
        shape["points"] = points;
    } else if (name == "text") {
        shape["op"] = "text";
        shape["color"] = attr("fill");
        shape["text"] = unescaped(element->content);
        fits = pick({{"x", "x"}, {"y", "y"}, {"font-size", "size"}});
    }
    if (!fits || shape_line(shape) != line) return kept;
    return shape;
}

/// The units of a drawing's svg line, when it is written as every runtime writes it.
std::optional<DrawingUnits> units_of(const std::string& line) {
    if (!line.starts_with(svg_start) || !line.ends_with("\">")) return std::nullopt;
    std::string sizes = line.substr(svg_start.size(), line.size() - svg_start.size() - 2);
    size_t space = sizes.find(' ');
    if (space == std::string::npos) return std::nullopt;
    auto w = int_of(sizes.substr(0, space));
    auto h = int_of(sizes.substr(space + 1));
    if (!w || !h) return std::nullopt;
    return DrawingUnits{*w, *h};
}

/// A shape's place in a drawing, counted from 1, as an action names it.
int64_t index_of(const json& value, const char* action) {
    if (!value.is_number_integer()) throw InvalidError(std::string("a canvas's ") + action + " does not fit: " + value.dump());
    return value.get<int64_t>();
}

double coordinate(const json& move, const char* name) {
    json value = field(move, name);
    if (value.is_null()) return 0;
    if (!value.is_number()) throw InvalidError("a canvas's move does not fit: " + move.dump());
    return value.get<double>();
}

json action_of(const json& action, const char* name) { return action.is_object() && action.contains(name) ? action[name] : json(nullptr); }

}  // namespace

CanvasArgs canvas_args_of(const json& args) {
    auto text = [&](const char* name) { return text_in(args, name); };
    CanvasArgs a;
    a.part = text("part");
    a.mode = text("mode");
    a.list = text("list");
    a.path = text("path");
    json at = field(args, "at");
    if (at.is_array() && at.size() == 2 && at[0].is_string() && at[1].is_string()) {
        a.at = {at[0].get<std::string>(), at[1].get<std::string>()};
    } else if (a.mode == "objects") {
        throw InvalidError("a canvas of objects names their x and y fields, as at: [\"x\", \"y\"]");
    }
    json units = field(args, "units");
    if (units.is_array() && units.size() == 2 && units[0].is_number_integer() && units[1].is_number_integer()) {
        a.units = DrawingUnits{units[0].get<int64_t>(), units[1].get<int64_t>()};
    } else if (!units.is_null()) {
        throw InvalidError("a canvas's units are a width and a height, as [400, 300], not " + units.dump());
    }
    json grid = field(args, "grid");
    if (grid.is_number()) a.grid = whole(grid.get<double>());
    a.snap = field(args, "snap") == json(true);
    a.readonly = field(args, "readonly") == json(true);
    return a;
}

bool canvas_event(WindowState& state, const CanvasArgs& a, const json& action) {
    if (a.readonly) return false;
    Place place = [&](double v) {
        if (a.snap && a.grid > 0) return whole(v / double(a.grid)) * a.grid;
        return whole(v);
    };
    if (a.mode == "objects") {
        json move = action_of(action, "move");
        if (move.is_null()) throw InvalidError("an object is moved");
        if (!move.is_object()) throw InvalidError("a canvas's move does not fit: " + move.dump());
        json id = field(move, "id");
        int64_t x = place(coordinate(move, "x")), y = place(coordinate(move, "y"));
        if (!found_at(state.data, a.list) || !value_at(state.data, a.list).is_array()) throw InvalidError("the window has no list " + a.list);
        json& list = place_at(state.data, a.list);
        for (size_t i = 0; i < list.size(); ++i) {
            json& record = list[i];
            if (!record.is_object() || field(record, "id") != id) continue;
            if (number_of(field(record, a.at[0])) == double(x) && number_of(field(record, a.at[1])) == double(y)) return false;
            record[a.at[0]] = x;
            record[a.at[1]] = y;
            // Chosen by its place in the list, as a table's row.
            state.selected[a.part] = int(i);
            return true;
        }
        return false;
    }
    if (a.mode == "paint") {
        json stroke = action_of(action, "stroke");
        if (stroke.is_null()) throw InvalidError("a painted canvas is stroked");
        // The picture as the image field holds a new one (conformance/pictures.json), kept as it comes.
        if (value_at(state.data, a.path) == stroke) return false;
        // A stroke that is no picture is refused: the record never holds it.
        if (!stroke.is_object() || !stroke.contains("data") || !stroke["data"].is_string() || !kind_of(from_base64(stroke["data"].get<std::string>()))) return false;
        place_at(state.data, a.path) = stroke;
        state.selected.erase(a.part);
        return true;
    }
    if (a.mode == "vector") {
        json was = value_at(state.data, a.path);
        std::string text = was.is_string() ? was.get<std::string>() : "";
        auto [units, shapes] = drawing_shapes(text, a.units.value_or(default_units));
        int64_t selected = -1;
        int64_t count = int64_t(shapes.size());
        if (json add = action_of(action, "add"); !add.is_null()) {
            if (!add.is_object()) throw InvalidError("a canvas's add does not fit: " + add.dump());
            shapes.push_back(fit_shape(add, place));
            selected = int64_t(shapes.size());
        } else if (json change = action_of(action, "change"); !change.is_null()) {
            if (!change.is_object()) throw InvalidError("a canvas's change does not fit: " + change.dump());
            int64_t index = index_of(field(change, "index"), "change");
            if (index < 1 || index > count) return false;
            json shape = field(change, "shape");
            if (!shape.is_object()) throw InvalidError("a canvas's change does not fit: " + change.dump());
            shapes[size_t(index - 1)] = fit_shape(shape, place);
            selected = index;
        } else if (json move = action_of(action, "move"); !move.is_null()) {
            if (!move.is_object()) throw InvalidError("a canvas's move does not fit: " + move.dump());
            int64_t index = index_of(field(move, "index"), "move");
            if (index < 1 || index > count || !moved_shape(shapes[size_t(index - 1)], place(coordinate(move, "x")), place(coordinate(move, "y")))) return false;
            selected = index;
        } else if (json gone = action_of(action, "delete"); !gone.is_null()) {
            int64_t index = index_of(gone, "delete");
            if (index < 1 || index > count) return false;
            shapes.erase(shapes.begin() + (index - 1));
        } else {
            throw InvalidError("a vector canvas adds, changes, moves or deletes a shape");
        }
        std::string drawn = drawing_text(units, shapes);
        if (drawn == text) return false;
        place_at(state.data, a.path) = drawn;
        // Chosen by its place in the drawing from 0, as a table's row.
        if (selected > 0) state.selected[a.part] = int(selected - 1);
        else state.selected.erase(a.part);
        return true;
    }
    throw InvalidError("a canvas's mode is objects, paint or vector, not \"" + a.mode + "\"");
}

std::string drawing_text(const DrawingUnits& units, const json& shapes) {
    std::string out = svg_start + std::to_string(units[0]) + " " + std::to_string(units[1]) + "\">\n";
    if (shapes.is_array())
        for (const auto& shape : shapes) out += shape_line(shape) + "\n";
    return out + "</svg>\n";
}

std::pair<DrawingUnits, json> drawing_shapes(const std::string& text, DrawingUnits units) {
    json shapes = json::array();
    size_t start = 0;
    while (start <= text.size()) {
        size_t end = text.find('\n', start);
        std::string line = text.substr(start, end == std::string::npos ? std::string::npos : end - start);
        start = end == std::string::npos ? text.size() + 1 : end + 1;
        if (line.find_first_not_of(" \t\r") == std::string::npos || line == "</svg>") continue;
        if (line.starts_with("<svg")) {
            if (auto read = units_of(line)) units = *read;
            continue;
        }
        shapes.push_back(read_shape(line));
    }
    return {units, shapes};
}

}  // namespace clarity
