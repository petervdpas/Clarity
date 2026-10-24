package clarity

// HTTP calls (DESIGN 5.23) against the cases every runtime shares
// (conformance/http.json), on a small server of the test's own.

import (
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
)

func TestEveryCallGivesTheReplyTheCasesName(t *testing.T) {
	text, err := os.ReadFile("../../../conformance/http.json")
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []struct {
			Name       string            `json:"name"`
			Method     string            `json:"method"`
			Path       string            `json:"path"`
			URL        string            `json:"url"`
			Body       string            `json:"body"`
			Headers    map[string]string `json:"headers"`
			Status     int64             `json:"status"`
			Ok         bool              `json:"ok"`
			Text       *string           `json:"text"`
			TextStarts string            `json:"text_starts"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		switch r.URL.Path {
		case "/hello":
			io.WriteString(w, "hello")
		case "/echo":
			body, _ := io.ReadAll(r.Body)
			io.WriteString(w, r.Method+" "+string(body))
		case "/header":
			io.WriteString(w, r.Header.Get("X-Test"))
		default:
			w.WriteHeader(http.StatusNotFound)
			io.WriteString(w, "no such page")
		}
	}))
	defer server.Close()
	for _, c := range file.Cases {
		url := c.URL
		if url == "" {
			url = server.URL + c.Path
		}
		var headers []Header
		for name, value := range c.Headers {
			headers = append(headers, Header{Name: name, Value: value})
		}
		reply := HTTP(c.Method, url, c.Body, headers)
		if reply.Status != c.Status || reply.Ok != c.Ok {
			t.Errorf("%s: status %d ok %v, want %d %v (%s)", c.Name, reply.Status, reply.Ok, c.Status, c.Ok, reply.Text)
		}
		if c.Text != nil && reply.Text != *c.Text {
			t.Errorf("%s: text %q, want %q", c.Name, reply.Text, *c.Text)
		}
		if c.TextStarts != "" && !strings.HasPrefix(reply.Text, c.TextStarts) {
			t.Errorf("%s: text %q, want it to start with %q", c.Name, reply.Text, c.TextStarts)
		}
	}
}

func TestAURLNamesItsHost(t *testing.T) {
	for url, host := range map[string]string{
		"https://api.example.com/orders?x=1": "api.example.com",
		"http://127.0.0.1:1/":                "127.0.0.1",
		"https://me:secret@example.com:8443": "example.com",
		"http://[::1]:8080/x":                "::1",
	} {
		if got := HTTPHost(url); got != host {
			t.Errorf("%s: %q, want %q", url, got, host)
		}
	}
}
