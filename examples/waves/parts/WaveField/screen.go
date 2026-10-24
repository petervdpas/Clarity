package wavefield

// screen is where the part's picture goes: the window Clarity gave it. The
// part fills a buffer of pixels, blue, green, red and a byte over, and the
// screen puts it on the window. One file per system does that and nothing
// else, so the drawing itself stays plain Go.
type screen interface {
	blit(pix []byte, width, height int)
	close()
}
