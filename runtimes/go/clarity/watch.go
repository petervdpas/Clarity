// What the screen sees of a long engine call and how the person stops it
// (DESIGN 5.12, `uses progress`): the shell watches the handler it runs,
// and every engine call the handler makes reports to that watch.

package clarity

import "sync"

// Watch is what the screen sees of a long call and how the person stops it
// (conformance/progress.json): every report of how far it is goes to
// Progress, and once Stop is closed the call ends, failing with "it was
// stopped".
type Watch struct {
	// Progress is told each report, done of total; nil drops them.
	Progress func(done, total int64)
	// Stop is closed to stop the call; nil never stops it.
	Stop <-chan struct{}
}

var (
	watchMu sync.Mutex
	current Watch
)

// Watching runs run with watch as the watch of every engine call it makes,
// as a shell runs a handler; handlers take turns, so one watch holds at a time.
func Watching(watch Watch, run func()) {
	watchMu.Lock()
	was := current
	current = watch
	watchMu.Unlock()
	defer func() {
		watchMu.Lock()
		current = was
		watchMu.Unlock()
	}()
	run()
}

// CurrentWatch is the watch of the handler running now; empty outside one.
func CurrentWatch() Watch {
	watchMu.Lock()
	defer watchMu.Unlock()
	return current
}
