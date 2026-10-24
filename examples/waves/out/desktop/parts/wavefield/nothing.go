package wavefield

// nothing is a screen that keeps the picture to itself.
type nothing struct{}

func (nothing) blit([]byte, int, int) {}
func (nothing) close()                {}
