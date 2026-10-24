//go:build windows

package wavefield

// Putting the buffer on the window, on Windows: one StretchDIBits of the
// whole picture into the part's own window. The standard library's syscall
// is enough, so the part needs nothing from outside Go.

import (
	"syscall"
	"unsafe"
)

var (
	user32 = syscall.NewLazyDLL("user32.dll")
	gdi32  = syscall.NewLazyDLL("gdi32.dll")

	procGetDC          = user32.NewProc("GetDC")
	procReleaseDC      = user32.NewProc("ReleaseDC")
	procStretchDIBits  = gdi32.NewProc("StretchDIBits")
	procSetStretchMode = gdi32.NewProc("SetStretchBltMode")
)

const (
	diRGBColors  = 0
	srcCopy      = 0x00CC0020
	biRGB        = 0
	colorOnColor = 3
)

// bitmapInfoHeader is Win32's BITMAPINFOHEADER.
type bitmapInfoHeader struct {
	size          uint32
	width         int32
	height        int32
	planes        uint16
	bitCount      uint16
	compression   uint32
	sizeImage     uint32
	xPelsPerMeter int32
	yPelsPerMeter int32
	clrUsed       uint32
	clrImportant  uint32
}

type windowScreen struct{ window uintptr }

func openScreen(w Window) screen { return &windowScreen{window: w.Handle} }

func (s *windowScreen) blit(pix []byte, width, height int) {
	if s.window == 0 || len(pix) == 0 {
		return
	}
	dc, _, _ := procGetDC.Call(s.window)
	if dc == 0 {
		return
	}
	defer procReleaseDC.Call(s.window, dc)
	procSetStretchMode.Call(dc, colorOnColor)
	info := bitmapInfoHeader{
		size:  uint32(unsafe.Sizeof(bitmapInfoHeader{})),
		width: int32(width),
		// Negative: the buffer's first row is the window's top row.
		height:      -int32(height),
		planes:      1,
		bitCount:    32,
		compression: biRGB,
	}
	procStretchDIBits.Call(dc,
		0, 0, uintptr(int32(width)), uintptr(int32(height)),
		0, 0, uintptr(int32(width)), uintptr(int32(height)),
		uintptr(unsafe.Pointer(&pix[0])), uintptr(unsafe.Pointer(&info)),
		diRGBColors, srcCopy)
}

func (s *windowScreen) close() {}
