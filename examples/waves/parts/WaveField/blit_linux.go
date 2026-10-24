//go:build linux && cgo

package wavefield

// Putting the buffer on the window, on Linux: one XPutImage of the whole
// picture into the part's own X11 window.

/*
#cgo LDFLAGS: -lX11
#include <X11/Xlib.h>
#include <stdlib.h>

static GC clarity_gc(Display* d, unsigned long window) {
    return XCreateGC(d, (Window)window, 0, NULL);
}

static void clarity_blit(Display* d, unsigned long window, GC gc, void* pix, int width, int height) {
    XImage* image = XCreateImage(d, DefaultVisual(d, DefaultScreen(d)), 24, ZPixmap, 0, (char*)pix, width, height, 32, 0);
    if (!image) return;
    XPutImage(d, (Window)window, gc, image, 0, 0, 0, 0, width, height);
    // The buffer is Go's: Xlib must not free it.
    image->data = NULL;
    XDestroyImage(image);
    XFlush(d);
}
*/
import "C"

import "unsafe"

type windowScreen struct {
	display *C.Display
	window  C.ulong
	gc      C.GC
}

func openScreen(w Window) screen {
	if w.Display == nil || w.Handle == 0 {
		return nothing{}
	}
	display := (*C.Display)(w.Display)
	window := C.ulong(w.Handle)
	return &windowScreen{display: display, window: window, gc: C.clarity_gc(display, window)}
}

func (s *windowScreen) blit(pix []byte, width, height int) {
	if len(pix) == 0 {
		return
	}
	C.clarity_blit(s.display, s.window, s.gc, unsafe.Pointer(&pix[0]), C.int(width), C.int(height))
}

func (s *windowScreen) close() {
	if s.gc != nil {
		C.XFreeGC(s.display, s.gc)
		s.gc = nil
	}
}
