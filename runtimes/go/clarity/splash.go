package clarity

import "sync"

// What the splash screen says is loading (splash.status = @loadingData):
// code sets it while the app's on start runs, and the screen asks for it a
// few times a second until the main window shows. It is a text as a message
// is one, so the screen shows a key in the person's language.

var splash struct {
	sync.Mutex
	status *Text
}

// SetSplashStatus says what loads now.
func SetSplashStatus(text Text) {
	splash.Lock()
	defer splash.Unlock()
	splash.status = &text
}

// SplashStatus is what code last said loads; nil while it said nothing.
func SplashStatus() *Text {
	splash.Lock()
	defer splash.Unlock()
	if splash.status == nil {
		return nil
	}
	text := *splash.status
	return &text
}
