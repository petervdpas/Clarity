//go:build linux && cgo

package parts

/*
#cgo LDFLAGS: -lX11
#include <X11/Xlib.h>
#include <stdlib.h>

static unsigned long clarity_child(Display* d, unsigned long parent, int x, int y, int w, int h) {
    Window c = XCreateSimpleWindow(d, (Window)parent, x, y, w > 0 ? w : 1, h > 0 ? h : 1, 0, 0, BlackPixel(d, DefaultScreen(d)));
    XFlush(d);
    return (unsigned long)c;
}
static int clarity_screen(Display* d) { return DefaultScreen(d); }
*/
import "C"

import (
	"errors"
	"unsafe"
)

// x11 is the host's X11 connection, used on its thread only.
type x11 struct{ display *C.Display }

func openPlatform() (windows, error) {
	d := C.XOpenDisplay(nil)
	if d == nil {
		return nil, errors.New("custom parts need X11, and no X display could be opened")
	}
	return &x11{display: d}, nil
}

func (x *x11) child(parent uint64, px, py, width, height int) uint64 {
	return uint64(C.clarity_child(x.display, C.ulong(parent), C.int(px), C.int(py), C.int(width), C.int(height)))
}

func (x *x11) window(child uint64) Window {
	return Window{XID: child, Display: unsafe.Pointer(x.display), Screen: int(C.clarity_screen(x.display))}
}

func (x *x11) move(child uint64, px, py, width, height int) {
	C.XMoveResizeWindow(x.display, C.Window(child), C.int(px), C.int(py), C.uint(max(width, 1)), C.uint(max(height, 1)))
}

func (x *x11) show(child uint64, shown bool) {
	if shown {
		C.XMapRaised(x.display, C.Window(child))
	} else {
		C.XUnmapWindow(x.display, C.Window(child))
	}
}

func (x *x11) destroy(child uint64) { C.XDestroyWindow(x.display, C.Window(child)) }
func (x *x11) flush()               { C.XFlush(x.display) }
func (x *x11) close()               { C.XCloseDisplay(x.display) }
