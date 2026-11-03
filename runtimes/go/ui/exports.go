package ui

// The Go side of the library's callbacks (callbacks.c): each takes the Go
// value behind its handle and answers. Nothing a program or a part panics
// with crosses into the library: it is said on standard error, and the
// screen goes on.

/*
#include "callbacks.h"
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"os"
	"runtime/cgo"
)

// safe runs a callback, saying what panicked in it.
func safe(what string, run func()) {
	defer func() {
		if r := recover(); r != nil {
			fmt.Fprintf(os.Stderr, "%s failed: %v\n", what, r)
		}
	}()
	run()
}

//export goProgramCall
func goProgramCall(user C.uintptr_t, command *C.char, commandLen C.size_t, args *C.char, argsLen C.size_t, answer *C.ClarityAnswer) {
	name := given(command, commandLen)
	safe(name, func() {
		p, ok := cgo.Handle(user).Value().(*Program)
		if !ok {
			return
		}
		answerWith(answer, p.Answer(name, []byte(given(args, argsLen)), thread()))
	})
}

//export goProgramRelease
func goProgramRelease(user C.uintptr_t) {
	safe("release", func() { cgo.Handle(user).Delete() })
}

//export goCustomMake
func goCustomMake(user C.uintptr_t) (made C.uintptr_t) {
	safe("a custom part's make", func() {
		make, ok := cgo.Handle(user).Value().(func() Part)
		if !ok {
			return
		}
		if part := make(); part != nil {
			made = C.uintptr_t(cgo.NewHandle(part))
		}
	})
	return made
}

// values reads a JSON array of values; none when it is no array.
func values(text string) []json.RawMessage {
	var out []json.RawMessage
	_ = json.Unmarshal([]byte(text), &out)
	return out
}

//export goCustomSet
func goCustomSet(handle C.uintptr_t, name *C.char, nameLen C.size_t, value *C.char, valueLen C.size_t, cx *C.ClarityCustomCx) {
	safe("a custom part's set", func() {
		if part := partOf(handle); part != nil {
			part.Set(given(name, nameLen), json.RawMessage(given(value, valueLen)), &Cx{cx})
		}
	})
}

//export goCustomHear
func goCustomHear(handle C.uintptr_t, hear *C.char, hearLen C.size_t, given_ *C.char, givenLen C.size_t, cx *C.ClarityCustomCx) {
	safe("a custom part's hear", func() {
		if part := partOf(handle); part != nil {
			part.Hear(given(hear, hearLen), values(given(given_, givenLen)), &Cx{cx})
		}
	})
}

//export goCustomDraw
func goCustomDraw(handle C.uintptr_t, canvas *C.ClarityCanvas, x, y, width, height C.double) {
	safe("a custom part's draw", func() {
		if part := partOf(handle); part != nil {
			part.Draw(&Canvas{canvas}, float64(x), float64(y), float64(width), float64(height))
		}
	})
}

//export goCustomStart
func goCustomStart(handle C.uintptr_t, cx *C.ClarityCustomCx) {
	safe("a custom part's start", func() {
		if part, ok := partOf(handle).(Starter); ok {
			part.Start(&Cx{cx})
		}
	})
}

//export goCustomFocusable
func goCustomFocusable(handle C.uintptr_t) (taken C.int32_t) {
	safe("a custom part's focusable", func() {
		if part, ok := partOf(handle).(Focusable); ok && part.Focusable() {
			taken = 1
		}
	})
	return taken
}

//export goCustomPointer
func goCustomPointer(handle C.uintptr_t, which C.int, x, y C.double, modifiers, clicks C.uint32_t, cx *C.ClarityCustomCx) {
	safe("a custom part's pointer", func() {
		part := partOf(handle)
		at := func(call func(x, y float64, modifiers, clicks uint32, cx *Cx)) {
			call(float64(x), float64(y), uint32(modifiers), uint32(clicks), &Cx{cx})
		}
		switch which {
		case 0:
			if p, ok := part.(Presser); ok {
				at(p.PointerPressed)
			}
		case 1:
			if p, ok := part.(Mover); ok {
				at(p.PointerMoved)
			}
		default:
			if p, ok := part.(Releaser); ok {
				at(p.PointerReleased)
			}
		}
	})
}

//export goCustomClick
func goCustomClick(handle C.uintptr_t, cx *C.ClarityCustomCx) {
	safe("a custom part's click", func() {
		if part, ok := partOf(handle).(Clicker); ok {
			part.Click(&Cx{cx})
		}
	})
}

//export goCustomWheel
func goCustomWheel(handle C.uintptr_t, dx, dy C.double, modifiers C.uint32_t, cx *C.ClarityCustomCx) (taken C.int32_t) {
	safe("a custom part's wheel", func() {
		if part, ok := partOf(handle).(Wheeler); ok && part.Wheel(float64(dx), float64(dy), uint32(modifiers), &Cx{cx}) {
			taken = 1
		}
	})
	return taken
}

//export goCustomKey
func goCustomKey(handle C.uintptr_t, name *C.char, nameLen C.size_t, modifiers C.uint32_t, cx *C.ClarityCustomCx) (taken C.int32_t) {
	safe("a custom part's key", func() {
		if part, ok := partOf(handle).(Keyer); ok && part.Key(given(name, nameLen), uint32(modifiers), &Cx{cx}) {
			taken = 1
		}
	})
	return taken
}

//export goCustomText
func goCustomText(handle C.uintptr_t, typed *C.char, typedLen C.size_t, cx *C.ClarityCustomCx) {
	safe("a custom part's text", func() {
		if part, ok := partOf(handle).(Typer); ok {
			part.Text(given(typed, typedLen), &Cx{cx})
		}
	})
}

//export goCustomFocusChanged
func goCustomFocusChanged(handle C.uintptr_t, focused C.int32_t, cx *C.ClarityCustomCx) {
	safe("a custom part's focus", func() {
		if part, ok := partOf(handle).(FocusTaker); ok {
			part.FocusChanged(focused != 0, &Cx{cx})
		}
	})
}

//export goCustomWake
func goCustomWake(handle C.uintptr_t, cx *C.ClarityCustomCx) {
	safe("a custom part's wake", func() {
		if part, ok := partOf(handle).(Waker); ok {
			part.Wake(&Cx{cx})
		}
	})
}

//export goCustomStop
func goCustomStop(handle C.uintptr_t) {
	safe("a custom part's stop", func() {
		if part, ok := partOf(handle).(Stopper); ok {
			part.Stop()
		}
	})
}

//export goCustomTooltip
func goCustomTooltip(handle C.uintptr_t, x, y C.double, tip *C.ClarityAnswer) {
	safe("a custom part's tooltip", func() {
		if part, ok := partOf(handle).(Tipper); ok {
			if said, has := part.TooltipAt(float64(x), float64(y)); has {
				answerWith(tip, []byte(said))
			}
		}
	})
}

//export goCustomDestroy
func goCustomDestroy(handle C.uintptr_t) {
	safe("a custom part's destroy", func() {
		if handle == 0 {
			return
		}
		part := partOf(handle)
		defer cgo.Handle(handle).Delete()
		if part, ok := part.(Destroyer); ok {
			part.Destroy()
		}
	})
}
