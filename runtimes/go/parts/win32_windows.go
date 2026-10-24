//go:build windows

package parts

// Native windows on Windows: Win32 child windows of the shell's own window,
// made on the host's goroutine, which owns them and answers their messages.
// A window that does not clip its siblings paints over whatever shares its
// place, and the webview is such a window, so the parent's children are made
// to clip before a part is put among them. No cgo: this calls user32 through
// golang.org/x/sys/windows, which the package already brings in.

import (
	"errors"
	"unsafe"

	// The package's own windows are the parts'; win is the system's.
	win "golang.org/x/sys/windows"
)

var (
	user32   = win.NewLazySystemDLL("user32.dll")
	gdi32    = win.NewLazySystemDLL("gdi32.dll")
	kernel32 = win.NewLazySystemDLL("kernel32.dll")

	procRegisterClassW  = user32.NewProc("RegisterClassW")
	procCreateWindowExW = user32.NewProc("CreateWindowExW")
	procDefWindowProcW  = user32.NewProc("DefWindowProcW")
	procDestroyWindow   = user32.NewProc("DestroyWindow")
	procShowWindow      = user32.NewProc("ShowWindow")
	procSetWindowPos    = user32.NewProc("SetWindowPos")
	procPeekMessageW    = user32.NewProc("PeekMessageW")
	procTranslateMsg    = user32.NewProc("TranslateMessage")
	procDispatchMsgW    = user32.NewProc("DispatchMessageW")
	procEnumChildWins   = user32.NewProc("EnumChildWindows")
	procGetParent       = user32.NewProc("GetParent")
	procGetWindowLongW  = user32.NewProc("GetWindowLongW")
	procSetWindowLongW  = user32.NewProc("SetWindowLongW")
	procLoadCursorW     = user32.NewProc("LoadCursorW")
	procGetStockObject  = gdi32.NewProc("GetStockObject")
	procGetModuleHandle = kernel32.NewProc("GetModuleHandleW")
)

const (
	wsChild         = 0x40000000
	wsClipSiblings  = 0x04000000
	wsClipChildren  = 0x02000000
	swpNoSize       = 0x0001
	swpNoMove       = 0x0002
	swpNoZOrder     = 0x0004
	swpNoActivate   = 0x0010
	swpFrameChanged = 0x0020
	swHide          = 0
	swShowNA        = 8
	pmRemove        = 0x0001
	blackBrush      = 4
	idcArrow        = 32512
	hwndTop         = 0
)

// gwlStyle is the window's style among its long words. It is negative, so it
// is a variable: a constant would not convert to uintptr.
var gwlStyle int32 = -16

// wndClassW is Win32's WNDCLASSW.
type wndClassW struct {
	style      uint32
	wndProc    uintptr
	clsExtra   int32
	wndExtra   int32
	instance   win.Handle
	icon       win.Handle
	cursor     win.Handle
	background win.Handle
	menuName   *uint16
	className  *uint16
}

// message is Win32's MSG.
type message struct {
	hwnd    win.Handle
	what    uint32
	wParam  uintptr
	lParam  uintptr
	time    uint32
	pt      struct{ x, y int32 }
	private uint32
}

// win32 are the part windows of this program, on the host's goroutine.
type win32 struct{ instance win.Handle }

var partClass = win.StringToUTF16Ptr("ClarityPart")

// windowProc lets every message take its usual course: the part draws the
// window itself.
func windowProc(window win.Handle, what uint32, w, l uintptr) uintptr {
	answer, _, _ := procDefWindowProcW.Call(uintptr(window), uintptr(what), w, l)
	return answer
}

func windowStyle(window uintptr) uintptr {
	style, _, _ := procGetWindowLongW.Call(window, uintptr(gwlStyle))
	return style
}

// clipSiblings makes a window of the parent's own children clip its
// siblings, so it leaves the part's place alone however the two are ordered.
func clipSiblings(window win.Handle, parent uintptr) uintptr {
	if owner, _, _ := procGetParent.Call(uintptr(window)); owner != parent {
		return 1
	}
	style := windowStyle(uintptr(window))
	if style != 0 && style&wsClipSiblings == 0 {
		procSetWindowLongW.Call(uintptr(window), uintptr(gwlStyle), style|wsClipSiblings)
		procSetWindowPos.Call(uintptr(window), 0, 0, 0, 0, 0, swpNoMove|swpNoSize|swpNoZOrder|swpFrameChanged)
	}
	return 1
}

func openPlatform() (windows, error) {
	instance, _, _ := procGetModuleHandle.Call(0)
	if instance == 0 {
		return nil, errors.New("custom parts need the program's own module, which Windows did not give")
	}
	cursor, _, _ := procLoadCursorW.Call(0, uintptr(idcArrow))
	brush, _, _ := procGetStockObject.Call(uintptr(blackBrush))
	class := wndClassW{
		wndProc:    win.NewCallback(windowProc),
		instance:   win.Handle(instance),
		cursor:     win.Handle(cursor),
		background: win.Handle(brush),
		className:  partClass,
	}
	// The class is registered once; a second window finds it there already.
	procRegisterClassW.Call(uintptr(unsafe.Pointer(&class)))
	return &win32{instance: win.Handle(instance)}, nil
}

func (w *win32) child(parent uint64, x, y, width, height int) uint64 {
	child, _, _ := procCreateWindowExW.Call(
		0,
		uintptr(unsafe.Pointer(partClass)),
		0,
		wsChild|wsClipSiblings|wsClipChildren,
		uintptr(int32(x)), uintptr(int32(y)),
		uintptr(int32(atLeast(width))), uintptr(int32(atLeast(height))),
		uintptr(parent), 0, uintptr(w.instance), 0,
	)
	if child == 0 {
		return 0
	}
	procEnumChildWins.Call(uintptr(parent), win.NewCallback(clipSiblings), uintptr(parent))
	// Above the webview, which fills the same parent.
	procSetWindowPos.Call(child, hwndTop, 0, 0, 0, 0, swpNoMove|swpNoSize|swpNoActivate)
	return uint64(child)
}

func (w *win32) window(child uint64) Window { return Window{Handle: uintptr(child)} }

func (w *win32) move(child uint64, x, y, width, height int) {
	procSetWindowPos.Call(uintptr(child), hwndTop, uintptr(int32(x)), uintptr(int32(y)),
		uintptr(int32(atLeast(width))), uintptr(int32(atLeast(height))), swpNoActivate)
}

func (w *win32) show(child uint64, shown bool) {
	// SW_SHOWNA: shown without taking the keyboard from the window.
	how := uintptr(swHide)
	if shown {
		how = swShowNA
	}
	procShowWindow.Call(uintptr(child), how)
}

func (w *win32) destroy(child uint64) { procDestroyWindow.Call(uintptr(child)) }

// flush answers the messages of the part windows, which belong to this
// goroutine. The host asks for it between frames, so they are answered about
// sixty times a second and the shell is never kept waiting.
func (w *win32) flush() {
	var m message
	for {
		got, _, _ := procPeekMessageW.Call(uintptr(unsafe.Pointer(&m)), 0, 0, 0, pmRemove)
		if got == 0 {
			return
		}
		procTranslateMsg.Call(uintptr(unsafe.Pointer(&m)))
		procDispatchMsgW.Call(uintptr(unsafe.Pointer(&m)))
	}
}

func (w *win32) close() {}

func atLeast(n int) int {
	if n > 0 {
		return n
	}
	return 1
}
