// Custom part WaveField: water drawn on the canvas Clarity.UI gives it.
// Waves run out from a few sources, meet, and the part colours every pixel by
// how high the water stands there. Keep the methods of Part (clarity.go),
// which follow the part's declaration.
//
// Everything that makes the picture is plain Go: the part wakes about 60
// times a second while its window shows, fills a buffer of pixels and draws
// it in its place.
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

// frame is how long the part waits between pictures: about 60 a second.
const frame = 1000.0 / 60

// fine is how many of the screen's units one pixel of the water covers:
// the picture is drawn at half the place's size and stretched to it.
const fine = 2

// source is one place the water is pushed, and how fast its rings run.
type source struct {
	x, y   float64
	length float64
	phase  float64
}

type part struct {
	enabled bool
	speed   int64
	sources int64

	pix     []byte
	width   int
	height  int
	springs []source
	random  *rand.Rand
	clock   time.Time
	run     float64
	// best is the highest the water has stood since it was last calmed, so
	// the part tells of a swell only when the water goes past it.
	best int64
}

// New makes the part when its window opens.
func New() Part {
	p := &part{
		enabled: true,
		speed:   4,
		sources: 3,
		random:  rand.New(rand.NewSource(time.Now().UnixNano())),
		clock:   time.Now(),
	}
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
			length: 20 + p.random.Float64()*45,
			phase:  p.random.Float64() * 2 * math.Pi,
		})
	}
}

// Start has the water run: the part wakes again and again while its window shows.
func (p *part) Start(cx Cx) {
	p.clock = time.Now()
	cx.WakeIn(frame)
}

// Wake moves the water on by the time since the last frame and draws it anew.
func (p *part) Wake(cx Cx) {
	now := time.Now()
	step := math.Min(now.Sub(p.clock).Seconds(), 0.1)
	p.clock = now
	if p.enabled {
		p.run += step * float64(p.speed)
	}
	_, _, width, height := cx.Bounds()
	stands := p.paint(max(int(width)/fine, 1), max(int(height)/fine, 1))
	cx.Invalidate()
	cx.WakeIn(frame)

	// A swell is worth telling of when the water goes higher than it has
	// stood since it was calmed, and high enough to be a swell at all.
	if stands >= swellAt && stands > p.best {
		p.best = stands
		Swelled(cx, stands)
	}
}

// paint fills the picture, width by height pixels: how high the highest
// water stands, from 0 to 255.
func (p *part) paint(width, height int) int64 {
	if width != p.width || height != p.height {
		p.width, p.height = width, height
		p.pix = make([]byte, width*height*4)
	}
	springs := p.springs[:min(int(p.sources), len(p.springs))]
	highest := 0.0
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			sum := 0.0
			for _, s := range springs {
				dx := float64(x) - s.x*float64(width)
				dy := float64(y) - s.y*float64(height)
				sum += math.Sin(math.Sqrt(dx*dx+dy*dy)/s.length*math.Pi - p.run*2 + s.phase)
			}
			// From the sum of the waves to one value from 0 to 1.
			level := (sum/float64(len(springs)) + 1) / 2
			highest = math.Max(highest, level)
			at := (y*width + x) * 4
			r, g, b := water(level, p.enabled)
			p.pix[at], p.pix[at+1], p.pix[at+2], p.pix[at+3] = r, g, b, 255
		}
	}
	return int64(highest * 255)
}

// Draw puts the picture in the part's place.
func (p *part) Draw(canvas Canvas, x, y, width, height float64) {
	if p.pix == nil {
		canvas.Fill(x, y, width, height, 0xFF061C48, 0)
		return
	}
	canvas.Pixels(p.pix, p.width, p.height, p.width*4, x, y, width, height)
}

// water is the colour of water standing at level, from 0 to 1: deep blue
// below, white foam on top. Water nobody tends keeps its colours but grey.
func water(level float64, enabled bool) (r, g, b byte) {
	level = math.Min(math.Max(level, 0), 1)
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

func (p *part) SetEnabled(enabled bool, cx Cx) { p.enabled = enabled }

func (p *part) SetSpeed(speed int64, cx Cx) {
	p.speed = min(max(speed, 1), 10)
}

func (p *part) SetSources(sources int64, cx Cx) {
	p.sources = min(max(sources, 1), most)
}

// Calm stills the water and starts it again from somewhere new.
func (p *part) Calm(cx Cx) {
	p.run = 0
	p.best = 0
	p.stir()
}
