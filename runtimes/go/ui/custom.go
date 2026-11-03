package ui

/*
#include <stdlib.h>
#include "callbacks.h"
*/
import "C"

import (
	"encoding/json"
	"runtime/cgo"
	"strings"
	"sync"
)

// A custom part (DESIGN 5.19, 7.9): a program's own kind of part, written in
// Go, which Clarity.UI lays out, gives the pointer, keys and the focus, and
// asks to draw on its canvas. The library holds each part by a handle from
// its making until it is gone. Every method runs on the screen's thread,
// one at a time; what a part panics with is said on standard error and the
// window goes on (fail safe per part).

// Part is what every custom part does: it takes its window's words (Set: what
// it binds as "value", "enabled" and its declared properties; Hear: what code
// tells it) and draws itself in its place.
type Part interface {
	Set(name string, value json.RawMessage, cx *Cx)
	Hear(hear string, values []json.RawMessage, cx *Cx)
	Draw(canvas *Canvas, x, y, width, height float64)
}

// What a part may do besides, each its own interface: a part that has the
// method gets the call, one that has not does nothing.
type (
	// Focusable says whether it takes the focus, and so keys.
	Focusable interface{ Focusable() bool }
	// Starter hears that its window opened, after its first values.
	Starter interface{ Start(cx *Cx) }
	// Presser hears the main button go down on it.
	Presser interface {
		PointerPressed(x, y float64, modifiers, clicks uint32, cx *Cx)
	}
	// Mover hears the pointer move over it, or anywhere while it holds the pointer.
	Mover interface {
		PointerMoved(x, y float64, modifiers, clicks uint32, cx *Cx)
	}
	// Releaser hears the main button go up after it was pressed on it.
	Releaser interface {
		PointerReleased(x, y float64, modifiers, clicks uint32, cx *Cx)
	}
	// Clicker hears a press and release with the main button.
	Clicker interface{ Click(cx *Cx) }
	// Wheeler hears the wheel turn over it: true when it took the turn.
	Wheeler interface {
		Wheel(dx, dy float64, modifiers uint32, cx *Cx) bool
	}
	// Keyer hears a key while it has the focus: true when it took the key.
	Keyer interface {
		Key(name string, modifiers uint32, cx *Cx) bool
	}
	// Typer hears text typed while it has the focus.
	Typer interface{ Text(text string, cx *Cx) }
	// FocusTaker hears that it got or lost the focus.
	FocusTaker interface{ FocusChanged(focused bool, cx *Cx) }
	// Waker hears the time it asked for come (Cx.WakeIn).
	Waker interface{ Wake(cx *Cx) }
	// Stopper hears that its window left the screen: it stops what it started.
	Stopper interface{ Stop() }
	// Tipper says what it says about a point when the pointer rests there.
	Tipper interface {
		TooltipAt(x, y float64) (string, bool)
	}
	// Destroyer hears that it is gone.
	Destroyer interface{ Destroy() }
)

// Keys held, as bits of modifiers.
const (
	Shift   uint32 = C.CLARITY_SHIFT
	Control uint32 = C.CLARITY_CONTROL
	Alt     uint32 = C.CLARITY_ALT
	Meta    uint32 = C.CLARITY_META
)

var (
	registeredMu sync.Mutex
	registered   = map[string]bool{}
)

// Register adds a custom part of kind (its declaration's name, "Gauge"),
// made by make each time a window shows one. A kind registers once.
func Register(kind string, make func() Part) error {
	registeredMu.Lock()
	defer registeredMu.Unlock()
	if registered[kind] {
		return nil
	}
	k, kl := text(kind)
	handle := cgo.NewHandle(make)
	if err := failed(C.clarity_go_custom_register(k, kl, C.uintptr_t(handle)), "clarity_custom_register"); err != nil {
		handle.Delete()
		return err
	}
	registered[kind] = true
	return nil
}

// Cx is what a part does through the library during a callback: say what
// changed and tell its window. It is valid only during the call it is given to.
type Cx struct {
	c *C.ClarityCustomCx
}

// Bounds is where it is on the screen.
func (cx *Cx) Bounds() (x, y, width, height float64) {
	var bx, by, bw, bh C.double
	C.clarity_custom_bounds(cx.c, &bx, &by, &bw, &bh)
	return float64(bx), float64(by), float64(bw), float64(bh)
}

// Enabled is whether the person may use it.
func (cx *Cx) Enabled() bool {
	var on C.int32_t
	C.clarity_custom_enabled(cx.c, &on)
	return on != 0
}

// Invalidate has it drawn again.
func (cx *Cx) Invalidate() { C.clarity_custom_invalidate(cx.c) }

// WakeIn has it wake once (Wake) after ms milliseconds.
func (cx *Cx) WakeIn(ms float64) { C.clarity_custom_wake_in(cx.c, C.double(ms)) }

// Animate has it drawn every frame while on, as for a picture that moves by the clock.
func (cx *Cx) Animate(on bool) { C.clarity_custom_animate(cx.c, flag(on)) }

// Focus gives it the focus.
func (cx *Cx) Focus() { C.clarity_custom_focus(cx.c) }

// Change says the person changed its value: what it binds takes it (one
// edit, which Ctrl+Z takes back) and its change handler runs.
func (cx *Cx) Change(value any) {
	written, err := json.Marshal(value)
	if err != nil {
		return
	}
	v, vl := text(string(written))
	C.clarity_custom_change(cx.c, v, vl)
}

// Tell tells tell with its values: the handler wired to it runs with them.
func (cx *Cx) Tell(tell string, values ...any) {
	if values == nil {
		values = []any{}
	}
	written, err := json.Marshal(values)
	if err != nil {
		return
	}
	t, tl := text(tell)
	v, vl := text(string(written))
	C.clarity_custom_tell(cx.c, t, tl, v, vl)
}

func flag(on bool) C.int32_t {
	if on {
		return 1
	}
	return 0
}

// Canvas is what a part's Draw draws on. Colors are 0xAARRGGBB, places in
// the screen's units.
type Canvas struct {
	c *C.ClarityCanvas
}

// Fill fills a box, its corners rounded by radius.
func (c *Canvas) Fill(x, y, width, height float64, color uint32, radius float64) {
	C.clarity_canvas_fill(c.c, C.double(x), C.double(y), C.double(width), C.double(height), C.uint32_t(color), C.double(radius))
}

// Stroke draws a box's edge.
func (c *Canvas) Stroke(x, y, width, height float64, color uint32, lineWidth, radius float64) {
	C.clarity_canvas_stroke(c.c, C.double(x), C.double(y), C.double(width), C.double(height), C.uint32_t(color), C.double(lineWidth), C.double(radius))
}

// Line draws a line.
func (c *Canvas) Line(x1, y1, x2, y2 float64, color uint32, lineWidth float64) {
	C.clarity_canvas_line(c.c, C.double(x1), C.double(y1), C.double(x2), C.double(y2), C.uint32_t(color), C.double(lineWidth))
}

// Oval draws an oval in a box, filled or its edge.
func (c *Canvas) Oval(x, y, width, height float64, color uint32, filled bool, lineWidth float64) {
	C.clarity_canvas_oval(c.c, C.double(x), C.double(y), C.double(width), C.double(height), C.uint32_t(color), flag(filled), C.double(lineWidth))
}

// Path draws an SVG path's data ("M0 0 L10 10 Z"), filled or stroked.
func (c *Canvas) Path(svg string, color uint32, filled bool, lineWidth float64) {
	s, sl := text(svg)
	C.clarity_canvas_path(c.c, s, sl, C.uint32_t(color), flag(filled), C.double(lineWidth))
}

// Text writes text in a box: align 0 left, 1 center, 2 right; size 0 is the look's text size.
func (c *Canvas) Text(words string, x, y, width, height float64, color uint32, size float64, bold bool, align int) {
	t, tl := text(words)
	C.clarity_canvas_text(c.c, t, tl, C.double(x), C.double(y), C.double(width), C.double(height), C.uint32_t(color), C.double(size), flag(bold), C.int32_t(align))
}

// Measure is how wide text is at a size (0: the look's text size).
func (c *Canvas) Measure(words string, size float64, bold bool) float64 {
	t, tl := text(words)
	var width C.double
	C.clarity_canvas_measure(c.c, t, tl, C.double(size), flag(bold), &width)
	return float64(width)
}

// Pixels draws RGBA pixels (unpremultiplied, rows of stride bytes) into a box.
func (c *Canvas) Pixels(rgba []byte, width, height, stride int, x, y, boxWidth, boxHeight float64) {
	if len(rgba) < stride*height {
		return
	}
	p, _ := bytesOf(rgba)
	C.clarity_canvas_pixels(c.c, p, C.uint32_t(width), C.uint32_t(height), C.size_t(stride), C.double(x), C.double(y), C.double(boxWidth), C.double(boxHeight))
}

// Clip clips what follows to a box until Restore.
func (c *Canvas) Clip(x, y, width, height float64) {
	C.clarity_canvas_clip(c.c, C.double(x), C.double(y), C.double(width), C.double(height))
}

// Restore ends the last Clip.
func (c *Canvas) Restore() { C.clarity_canvas_restore(c.c) }

// LookColor is a color of the look by its role ("accent", "text", "border").
func (c *Canvas) LookColor(role string) uint32 {
	r, rl := text(role)
	var color C.uint32_t
	C.clarity_canvas_look_color(c.c, r, rl, &color)
	return uint32(color)
}

// partOf is the part behind a handle the library gave; nil when it is gone.
func partOf(handle C.uintptr_t) Part {
	if handle == 0 {
		return nil
	}
	part, _ := cgo.Handle(handle).Value().(Part)
	return part
}

// Integer is a value a part is given as a whole number: from a number or
// the text of one; 0 for anything else.
func Integer(value json.RawMessage) int64 {
	var number json.Number
	if json.Unmarshal(value, &number) == nil {
		if whole, err := number.Int64(); err == nil {
			return whole
		}
		if real, err := number.Float64(); err == nil {
			return int64(real)
		}
	}
	var text string
	if json.Unmarshal(value, &text) == nil {
		var whole json.Number = json.Number(strings.TrimSpace(text))
		if n, err := whole.Int64(); err == nil {
			return n
		}
	}
	return 0
}

// Bool is a value a part is given as true or false; anything else is false.
func Bool(value json.RawMessage) bool {
	var on bool
	_ = json.Unmarshal(value, &on)
	return on
}

// TextOf is a value a part is given as text in its plain form: a number as
// written, nothing as empty.
func TextOf(value json.RawMessage) string {
	var text string
	if json.Unmarshal(value, &text) == nil {
		return text
	}
	if len(value) == 0 || string(value) == "null" {
		return ""
	}
	return string(value)
}

// At is the value at i of a hear's values; none past the last.
func At(values []json.RawMessage, i int) json.RawMessage {
	if i < len(values) {
		return values[i]
	}
	return nil
}
