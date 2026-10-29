// Progress of a long engine call and Stop, against the cases every host
// shares (conformance/progress.json).

package engine

import (
	"encoding/json"
	"os"
	"reflect"
	"sync"
	"testing"
)

type progressCases struct {
	Name      string   `json:"name"`
	Functions []string `json:"functions"`
	Uses      []string `json:"uses"`
	Limits    Limits   `json:"limits"`
	Reported  []struct {
		Function string     `json:"function"`
		Args     []any      `json:"args"`
		Gives    any        `json:"gives"`
		Progress [][2]int64 `json:"progress"`
	} `json:"reported"`
	Stopped []struct {
		Function     string     `json:"function"`
		Args         []any      `json:"args"`
		StopAt       int64      `json:"stopAt"`
		FirstReports [][2]int64 `json:"firstReports"`
		Fails        string     `json:"fails"`
	} `json:"stopped"`
	Refused struct {
		Uses    []string `json:"uses"`
		Message string   `json:"message"`
	} `json:"refused"`
}

func loadProgress(t *testing.T) (progressCases, []byte) {
	t.Helper()
	var cases progressCases
	text, err := os.ReadFile("../../../conformance/progress.json")
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(text, &cases); err != nil {
		t.Fatal(err)
	}
	long, err := os.ReadFile("../../../conformance/engines/long.wasm")
	if err != nil {
		t.Fatal(err)
	}
	return cases, long
}

// watching keeps every report and closes Stop at report stopAt (0: never by a report).
func watching(stopAt int64) (Watch, func() [][2]int64) {
	var mu sync.Mutex
	var reports [][2]int64
	stop := make(chan struct{})
	var once sync.Once
	if stopAt == 0 {
		// Never stopped by a report; the cases that stop at 0 stop before the call.
		stopAt = -1
	}
	watch := Watch{
		Progress: func(done, total int64) {
			mu.Lock()
			reports = append(reports, [2]int64{done, total})
			mu.Unlock()
			if done == stopAt {
				once.Do(func() { close(stop) })
			}
		},
		Stop: stop,
	}
	return watch, func() [][2]int64 {
		mu.Lock()
		defer mu.Unlock()
		return append([][2]int64{}, reports...)
	}
}

func TestALongCallReportsHowFarItIs(t *testing.T) {
	cases, long := loadProgress(t)
	e, err := Load(cases.Name, long, Declaration{Functions: cases.Functions, Uses: cases.Uses, Limits: cases.Limits})
	if err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Reported {
		watch, reports := watching(0)
		got, err := e.CallWatched(0, watch, c.Function, c.Args...)
		if err != nil {
			t.Fatalf("%v: %v", c, err)
		}
		if g, _ := json.Marshal(got); string(g) != mustJSON(c.Gives) {
			t.Errorf("%v: gave %s", c, g)
		}
		if r := reports(); !reflect.DeepEqual(r, c.Progress) && !(len(r) == 0 && len(c.Progress) == 0) {
			t.Errorf("%v: reported %v", c, r)
		}
	}
}

func TestStopEndsALongCall(t *testing.T) {
	cases, long := loadProgress(t)
	e, err := Load(cases.Name, long, Declaration{Functions: cases.Functions, Uses: cases.Uses, Limits: cases.Limits})
	if err != nil {
		t.Fatal(err)
	}
	for _, c := range cases.Stopped {
		watch, reports := watching(c.StopAt)
		if c.StopAt == 0 {
			stopped := make(chan struct{})
			close(stopped)
			watch.Stop = stopped
		}
		_, err := e.CallWatched(0, watch, c.Function, c.Args...)
		if err == nil || err.Error() != c.Fails {
			t.Fatalf("%v: %v", c, err)
		}
		if r := reports(); len(r) < len(c.FirstReports) || !reflect.DeepEqual(r[:len(c.FirstReports)], c.FirstReports) && len(c.FirstReports) > 0 {
			t.Errorf("%v: reported %v", c, r)
		}
	}
}

func TestProgressIsAServiceTheEngineMustUse(t *testing.T) {
	cases, long := loadProgress(t)
	_, err := Load(cases.Name, long, Declaration{Functions: cases.Functions, Uses: cases.Refused.Uses, Limits: cases.Limits})
	if err == nil || err.Error() != cases.Refused.Message {
		t.Fatalf("refused with %v", err)
	}
}

func mustJSON(v any) string {
	b, _ := json.Marshal(v)
	return string(b)
}
