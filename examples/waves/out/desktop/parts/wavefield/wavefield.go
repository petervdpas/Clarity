// Custom part WaveField: water drawn in the native window Clarity gives it.
// Waves run out from a few sources, meet, and the part colours every pixel by
// how high the water stands there. Keep the methods of Part (clarity.go),
// which follow the part's declaration.
//
// Everything that makes the picture is plain Go: the part fills a buffer of
// pixels and asks the screen to put it on the window. Only that last step is
// written per system (blit_windows.go, blit_linux.go), so the drawing itself
// is the same everywhere.
package wavefield

import (
	"math"
	"math/rand"
	"time"
)

// most is the most sources the water can have, as the declaration says.
const most = 5

// swellAt is how high the water must stand before the part tells of it.
const swellAt = 240

// source is one place the water is pushed, and how fast its rings run.
type source struct {
	x, y   float64
	length float64
	phase  float64
}

type part struct {
	host    Host
	window  Window
	width   int
	height  int
	enabled bool
	speed   int64
	sources int64

	screen  screen
	pix     []byte
	springs []source
	random  *rand.Rand
	clock   time.Time
	run     float64
	// best is the highest the water has stood since it was last calmed, so
	// the part tells of a swell only when the water goes past it.
	best int64
}

// New makes the part in window, width by height pixels.
func New(window Window, width, height int, host Host) Part {
	p := &part{
		host:    host,
		window:  window,
		enabled: true,
		speed:   4,
		sources: 3,
		random:  rand.New(rand.NewSource(time.Now().UnixNano())),
		clock:   time.Now(),
	}
	p.screen = openScreen(window)
	p.Resize(width, height)
	p.stir()
	return p
}

// stir puts the sources somewhere new.
func (p *part) stir() {
	p.springs = p.springs[:0]
	for i := 0; i < most; i++ {
		p.springs = append(p.springs, source{
			x:      p.random.Float64(),
			y:      p.random.Float64(),
			length: 40 + p.random.Float64()*90,
			phase:  p.random.Float64() * 2 * math.Pi,
		})
	}
}

func (p *part) Resize(width, height int) {
	if width < 1 {
		width = 1
	}
	if height < 1 {
		height = 1
	}
	p.width, p.height = width, height
	p.pix = make([]byte, width*height*4)
}

func (p *part) Frame() {
	now := time.Now()
	step := now.Sub(p.clock).Seconds()
	p.clock = now
	if step > 0.1 {
		step = 0.1
	}
	if p.enabled {
		p.run += step * float64(p.speed)
	}

	springs := p.springs[:min(int(p.sources), len(p.springs))]
	highest := 0.0
	for y := 0; y < p.height; y++ {
		for x := 0; x < p.width; x++ {
			height := 0.0
			for _, s := range springs {
				dx := float64(x) - s.x*float64(p.width)
				dy := float64(y) - s.y*float64(p.height)
				height += math.Sin(math.Sqrt(dx*dx+dy*dy)/s.length*math.Pi - p.run*2 + s.phase)
			}
			// From the sum of the waves to one value from 0 to 1.
			level := (height/float64(len(springs)) + 1) / 2
			if level > highest {
				highest = level
			}
			at := (y*p.width + x) * 4
			r, g, b := water(level, p.enabled)
			// The buffer is blue, green, red, nothing: what both systems take.
			p.pix[at], p.pix[at+1], p.pix[at+2], p.pix[at+3] = b, g, r, 0
		}
	}
	p.screen.blit(p.pix, p.width, p.height)

	// A swell is worth telling of when the water goes higher than it has
	// stood since it was calmed, and high enough to be a swell at all.
	stands := int64(highest * 255)
	if stands >= swellAt && stands > p.best {
		p.best = stands
		p.host.Swelled(stands)
	}
}

// water is the colour of water standing at level, from 0 to 1: deep blue
// below, white foam on top. Water nobody tends keeps its colours but grey.
func water(level float64, enabled bool) (r, g, b byte) {
	if level < 0 {
		level = 0
	} else if level > 1 {
		level = 1
	}
	if !enabled {
		grey := byte(60 + level*120)
		return grey, grey, grey
	}
	// Deep (6, 28, 72) through sea (32, 120, 190) to foam (235, 250, 255).
	if level < 0.7 {
		at := level / 0.7
		return byte(6 + at*26), byte(28 + at*92), byte(72 + at*118)
	}
	at := (level - 0.7) / 0.3
	return byte(32 + at*203), byte(120 + at*130), byte(190 + at*65)
}

func (p *part) SetEnabled(enabled bool) { p.enabled = enabled }

func (p *part) SetSpeed(speed int64) {
	p.speed = min(max(speed, 1), 10)
}

func (p *part) SetSources(sources int64) {
	p.sources = min(max(sources, 1), most)
}

// Calm stills the water and starts it again from somewhere new.
func (p *part) Calm() {
	p.run = 0
	p.best = 0
	p.stir()
}
