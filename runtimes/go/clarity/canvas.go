package clarity

// Canvas modes (DESIGN 5.31), decided here in the runtime: the screen sends
// what the person did on a canvas (an object dropped, a stroke painted, a
// shape added, changed, moved or deleted) and the runtime changes the
// window's data, snapped to the canvas's grid, and chooses what it touched.
// A vector drawing is kept as SVG text in one plain form, which every runtime
// writes and reads alike (conformance/canvas-modes.json).

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"math"
	"reflect"
	"strconv"
	"strings"
)

// CanvasArgs are a canvas's mode and the mode's parameters, as a
// canvas_event carries them.
type CanvasArgs struct {
	Part     string    `json:"part"`
	Mode     string    `json:"mode"`
	List     string    `json:"list"`
	At       [2]string `json:"at"`
	Path     string    `json:"path"`
	Units    *[2]int   `json:"units"`
	Grid     int       `json:"grid"`
	Snap     bool      `json:"snap"`
	Readonly bool      `json:"readonly"`
}

// defaultUnits are a drawing's units when the canvas names none.
var defaultUnits = [2]int{1000, 1000}

const svgHead = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 %d %d">`

// CanvasEvent applies the person's action on a canvas to state, and says
// whether it changed anything; an action that changes nothing, or names no
// object or shape, is refused (false), as every action on a readonly canvas.
func CanvasEvent(state WindowState, a CanvasArgs, action map[string]json.RawMessage) (bool, error) {
	if a.Readonly {
		return false, nil
	}
	if state.Selected == nil {
		return false, &InvalidError{"a canvas event needs the window's selection"}
	}
	place := func(v float64) int64 {
		if a.Snap && a.Grid > 0 {
			return int64(math.Round(v/float64(a.Grid))) * int64(a.Grid)
		}
		return int64(math.Round(v))
	}
	read := func(name string, into any) error {
		if err := json.Unmarshal(action[name], into); err != nil {
			return &InvalidError{fmt.Sprintf("a canvas's %s does not fit: %s", name, action[name])}
		}
		return nil
	}
	switch a.Mode {
	case "objects":
		if action["move"] == nil {
			return false, &InvalidError{"an object is moved"}
		}
		var move struct {
			ID any `json:"id"`
		}
		var at struct {
			X float64 `json:"x"`
			Y float64 `json:"y"`
		}
		if err := read("move", &move); err != nil {
			return false, err
		}
		if err := read("move", &at); err != nil {
			return false, err
		}
		value, _ := ValueAt(state.Data, a.List)
		list, ok := value.([]any)
		if !ok {
			return false, &InvalidError{fmt.Sprintf("the window has no list %s", a.List)}
		}
		for i, row := range list {
			record, ok := row.(map[string]any)
			if !ok || !sameID(record["id"], move.ID) {
				continue
			}
			x, y := place(at.X), place(at.Y)
			if sameNumber(record[a.At[0]], x) && sameNumber(record[a.At[1]], y) {
				return false, nil
			}
			record[a.At[0]], record[a.At[1]] = x, y
			// Chosen as a table's row is: by its place in the list.
			state.Selected[a.Part] = i
			return true, nil
		}
		return false, nil
	case "paint":
		if action["stroke"] == nil {
			return false, &InvalidError{"a painted canvas is stroked"}
		}
		// The picture as the image field holds a new one (conformance/pictures.json).
		var picture any
		if err := read("stroke", &picture); err != nil {
			return false, err
		}
		if was, _ := ValueAt(state.Data, a.Path); reflect.DeepEqual(asJSON(was), picture) {
			return false, nil
		}
		// A stroke that is no picture is refused: the record never holds it.
		if !isPicture(picture) {
			return false, nil
		}
		if err := SetValueAt(state.Data, a.Path, picture); err != nil {
			return false, err
		}
		delete(state.Selected, a.Part)
		return true, nil
	case "vector":
		units := defaultUnits
		if a.Units != nil {
			units = *a.Units
		}
		was, _ := ValueAt(state.Data, a.Path)
		text, _ := was.(string)
		units, shapes := DrawingShapes(text, units)
		selected := -1
		switch {
		case action["add"] != nil:
			var shape map[string]any
			if err := read("add", &shape); err != nil {
				return false, err
			}
			fitted, err := fitShape(shape, place)
			if err != nil {
				return false, err
			}
			shapes = append(shapes, fitted)
			selected = len(shapes)
		case action["change"] != nil:
			var change struct {
				Index int            `json:"index"`
				Shape map[string]any `json:"shape"`
			}
			if err := read("change", &change); err != nil {
				return false, err
			}
			if change.Index < 1 || change.Index > len(shapes) {
				return false, nil
			}
			fitted, err := fitShape(change.Shape, place)
			if err != nil {
				return false, err
			}
			shapes[change.Index-1] = fitted
			selected = change.Index
		case action["move"] != nil:
			var move struct {
				Index int     `json:"index"`
				X     float64 `json:"x"`
				Y     float64 `json:"y"`
			}
			if err := read("move", &move); err != nil {
				return false, err
			}
			if move.Index < 1 || move.Index > len(shapes) || !movedShape(asShape(shapes[move.Index-1]), place(move.X), place(move.Y)) {
				return false, nil
			}
			selected = move.Index
		case action["delete"] != nil:
			var index int
			if err := read("delete", &index); err != nil {
				return false, err
			}
			if index < 1 || index > len(shapes) {
				return false, nil
			}
			shapes = append(shapes[:index-1], shapes[index:]...)
		default:
			return false, &InvalidError{"a vector canvas adds, changes, moves or deletes a shape"}
		}
		drawn := DrawingText(units, shapes)
		if drawn == text {
			return false, nil
		}
		if err := SetValueAt(state.Data, a.Path, drawn); err != nil {
			return false, err
		}
		if selected > 0 {
			state.Selected[a.Part] = selected - 1
		} else {
			delete(state.Selected, a.Part)
		}
		return true, nil
	}
	return false, &InvalidError{fmt.Sprintf("a canvas's mode is objects, paint or vector, not %q", a.Mode)}
}

// isPicture says whether a stroke is a new picture: {data: base64 of a picture's bytes}.
func isPicture(value any) bool {
	fields, _ := value.(map[string]any)
	data, _ := fields["data"].(string)
	bytes, err := base64.StdEncoding.DecodeString(data)
	if err != nil {
		return false
	}
	_, ok := KindOf(bytes)
	return ok
}

func asShape(value any) map[string]any {
	shape, _ := value.(map[string]any)
	if shape == nil {
		shape = map[string]any{}
	}
	return shape
}

func sameNumber(value any, n int64) bool {
	got, ok := chartNumber(value)
	return ok && got == float64(n)
}

// shapeFields are the numbers of each shape a vector canvas keeps, and which of them are places.
var shapeFields = map[string]struct{ numbers, places []string }{
	"rect":    {[]string{"x", "y", "width", "height"}, []string{"x", "y"}},
	"line":    {[]string{"x1", "y1", "x2", "y2", "width"}, []string{"x1", "y1", "x2", "y2"}},
	"ellipse": {[]string{"x", "y", "rx", "ry"}, []string{"x", "y"}},
	"path":    {[]string{"width"}, nil},
	"text":    {[]string{"x", "y", "size"}, []string{"x", "y"}},
}

// shapeDefaults are what a shape's left-out numbers are (conformance/drawing.json).
var shapeDefaults = map[string]float64{"line.width": 1, "path.width": 1, "text.size": 10}

// fitShape is a shape as a vector canvas keeps it: whole numbers, places snapped.
func fitShape(shape map[string]any, place func(float64) int64) (map[string]any, error) {
	op, _ := shape["op"].(string)
	fields, known := shapeFields[op]
	if !known {
		return nil, &InvalidError{fmt.Sprintf("a vector canvas keeps rect, line, ellipse, path and text, not %q", op)}
	}
	out := map[string]any{"op": op}
	for _, name := range fields.numbers {
		n, ok := chartNumber(shape[name])
		if !ok {
			n = shapeDefaults[op+"."+name]
		}
		if containsName(fields.places, name) {
			out[name] = place(n)
		} else {
			out[name] = int64(math.Round(n))
		}
	}
	color, _ := shape["color"].(string)
	out["color"] = color
	if op == "path" {
		points, _ := shape["points"].([]any)
		kept := make([]any, len(points))
		for i, p := range points {
			n, _ := chartNumber(p)
			kept[i] = place(n)
		}
		out["points"] = kept
	}
	if op == "text" {
		text, _ := shape["text"].(string)
		out["text"] = text
	}
	return out, nil
}

func containsName(names []string, name string) bool {
	for _, n := range names {
		if n == name {
			return true
		}
	}
	return false
}

// movedShape moves shape so its first place is at x, y, the rest with it;
// false when it is there already or is a line the runtime does not read.
func movedShape(shape map[string]any, x, y int64) bool {
	op, _ := shape["op"].(string)
	num := func(name string) int64 { n, _ := chartNumber(shape[name]); return int64(n) }
	var dx, dy int64
	switch op {
	case "rect", "ellipse", "text":
		dx, dy = x-num("x"), y-num("y")
		shape["x"], shape["y"] = x, y
	case "line":
		dx, dy = x-num("x1"), y-num("y1")
		shape["x1"], shape["y1"] = x, y
		shape["x2"], shape["y2"] = num("x2")+dx, num("y2")+dy
	case "path":
		points, _ := shape["points"].([]any)
		if len(points) < 2 {
			return false
		}
		first, _ := chartNumber(points[0])
		second, _ := chartNumber(points[1])
		dx, dy = x-int64(first), y-int64(second)
		for i, p := range points {
			n, _ := chartNumber(p)
			if i%2 == 0 {
				points[i] = int64(n) + dx
			} else {
				points[i] = int64(n) + dy
			}
		}
	default:
		return false
	}
	return dx != 0 || dy != 0
}

var svgEscapes = strings.NewReplacer("&", "&amp;", "<", "&lt;", ">", "&gt;", `"`, "&quot;")
var svgUnescapes = strings.NewReplacer("&lt;", "<", "&gt;", ">", "&quot;", `"`, "&amp;", "&")

// DrawingText is a drawing's SVG text: its units and its shapes, one element on a line.
func DrawingText(units [2]int, shapes []any) string {
	var out strings.Builder
	fmt.Fprintf(&out, svgHead+"\n", units[0], units[1])
	for _, s := range shapes {
		shape, _ := s.(map[string]any)
		out.WriteString(shapeLine(shape))
		out.WriteString("\n")
	}
	out.WriteString("</svg>\n")
	return out.String()
}

func shapeLine(shape map[string]any) string {
	n := func(name string) string {
		v, _ := chartNumber(shape[name])
		return strconv.FormatInt(int64(math.Round(v)), 10)
	}
	text := func(name string) string { v, _ := shape[name].(string); return svgEscapes.Replace(v) }
	switch shape["op"] {
	case "rect":
		return fmt.Sprintf(`<rect x="%s" y="%s" width="%s" height="%s" fill="%s"/>`, n("x"), n("y"), n("width"), n("height"), text("color"))
	case "line":
		return fmt.Sprintf(`<line x1="%s" y1="%s" x2="%s" y2="%s" stroke="%s" stroke-width="%s"/>`, n("x1"), n("y1"), n("x2"), n("y2"), text("color"), n("width"))
	case "ellipse":
		return fmt.Sprintf(`<ellipse cx="%s" cy="%s" rx="%s" ry="%s" fill="%s"/>`, n("x"), n("y"), n("rx"), n("ry"), text("color"))
	case "path":
		points, _ := shape["points"].([]any)
		pairs := []string{}
		for i := 0; i+1 < len(points); i += 2 {
			x, _ := chartNumber(points[i])
			y, _ := chartNumber(points[i+1])
			pairs = append(pairs, fmt.Sprintf("%d,%d", int64(math.Round(x)), int64(math.Round(y))))
		}
		return fmt.Sprintf(`<polyline points="%s" fill="none" stroke="%s" stroke-width="%s"/>`, strings.Join(pairs, " "), text("color"), n("width"))
	case "text":
		return fmt.Sprintf(`<text x="%s" y="%s" fill="%s" font-size="%s">%s</text>`, n("x"), n("y"), text("color"), n("size"), text("text"))
	}
	kept, _ := shape["text"].(string)
	return kept
}

// DrawingShapes reads a drawing's SVG text: its units (units when it names
// none) and its shapes. A line it does not read as a shape is kept as it is.
func DrawingShapes(text string, units [2]int) ([2]int, []any) {
	shapes := []any{}
	for _, line := range strings.Split(text, "\n") {
		if strings.TrimSpace(line) == "" || line == "</svg>" {
			continue
		}
		if strings.HasPrefix(line, "<svg") {
			var w, h int
			if _, err := fmt.Sscanf(line, svgHead, &w, &h); err == nil {
				units = [2]int{w, h}
			}
			continue
		}
		shapes = append(shapes, readShape(line))
	}
	return units, shapes
}

// readShape is the shape of one line of a drawing: written back, it must be
// the line again, or the line is kept as it is.
func readShape(line string) map[string]any {
	kept := map[string]any{"op": "svg", "text": line}
	name, attrs, content, ok := svgElement(line)
	if !ok {
		return kept
	}
	num := func(attr string) (int64, bool) {
		n, err := strconv.ParseInt(attrs[attr], 10, 64)
		return n, err == nil
	}
	shape := map[string]any{}
	pick := func(pairs ...string) bool {
		for i := 0; i < len(pairs); i += 2 {
			n, ok := num(pairs[i])
			if !ok {
				return false
			}
			shape[pairs[i+1]] = n
		}
		return true
	}
	fits := false
	switch name {
	case "rect":
		shape["op"], shape["color"] = "rect", attrs["fill"]
		fits = pick("x", "x", "y", "y", "width", "width", "height", "height")
	case "line":
		shape["op"], shape["color"] = "line", attrs["stroke"]
		fits = pick("x1", "x1", "y1", "y1", "x2", "x2", "y2", "y2", "stroke-width", "width")
	case "ellipse":
		shape["op"], shape["color"] = "ellipse", attrs["fill"]
		fits = pick("cx", "x", "cy", "y", "rx", "rx", "ry", "ry")
	case "polyline":
		shape["op"], shape["color"] = "path", attrs["stroke"]
		points := []any{}
		fits = pick("stroke-width", "width")
		for _, pair := range strings.Fields(attrs["points"]) {
			x, y, found := strings.Cut(pair, ",")
			nx, errX := strconv.ParseInt(x, 10, 64)
			ny, errY := strconv.ParseInt(y, 10, 64)
			if !found || errX != nil || errY != nil {
				fits = false
			}
			points = append(points, nx, ny)
		}
		shape["points"] = points
	case "text":
		shape["op"], shape["color"], shape["text"] = "text", attrs["fill"], svgUnescapes.Replace(content)
		fits = pick("x", "x", "y", "y", "font-size", "size")
	}
	if !fits || shapeLine(shape) != line {
		return kept
	}
	return shape
}

// svgElement reads `<name a="1" b="2"/>` or `<name a="1">content</name>`.
func svgElement(line string) (string, map[string]string, string, bool) {
	if !strings.HasPrefix(line, "<") {
		return "", nil, "", false
	}
	rest := line[1:]
	end := strings.IndexAny(rest, " />")
	if end <= 0 {
		return "", nil, "", false
	}
	name := rest[:end]
	rest = rest[end:]
	attrs := map[string]string{}
	for {
		rest = strings.TrimLeft(rest, " ")
		if strings.HasPrefix(rest, "/>") {
			return name, attrs, "", rest == "/>"
		}
		if strings.HasPrefix(rest, ">") {
			content, closed := strings.CutSuffix(rest[1:], "</"+name+">")
			return name, attrs, content, closed
		}
		eq := strings.Index(rest, `="`)
		if eq <= 0 {
			return "", nil, "", false
		}
		close := strings.Index(rest[eq+2:], `"`)
		if close < 0 {
			return "", nil, "", false
		}
		attrs[rest[:eq]] = svgUnescapes.Replace(rest[eq+2 : eq+2+close])
		rest = rest[eq+2+close+1:]
	}
}
