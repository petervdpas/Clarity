package clarity

// HTTP calls (DESIGN 5.23) against the cases every runtime shares
// (conformance/http.json), on a small server of the test's own.

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"strconv"
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
			Base       string            `json:"base"`
			Query      [][2]string       `json:"query"`
			Auth       HTTPAuth          `json:"auth"`
			Paging     *HTTPPaging       `json:"paging"`
		} `json:"cases"`
		Env map[string]string `json:"env"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	var server *httptest.Server
	server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		number := func(name string, otherwise int) int {
			if n, err := strconv.Atoi(r.URL.Query().Get(name)); err == nil {
				return n
			}
			return otherwise
		}
		// The numbers from `from` up to `to`, and at most 5, as a JSON list.
		numbers := func(from, to int) string {
			var out []string
			for n := from; n <= min(to, 5); n++ {
				out = append(out, strconv.Itoa(n))
			}
			return "[" + strings.Join(out, ",") + "]"
		}
		switch r.URL.Path {
		case "/paged":
			page, size := number("page", 1), number("size", 5)
			io.WriteString(w, numbers((page-1)*size+1, page*size))
		case "/offset":
			offset, limit := number("offset", 0), number("limit", 5)
			io.WriteString(w, `{"data":`+numbers(offset+1, offset+limit)+`}`)
		case "/cursor":
			after := number("after", 0)
			next := "null"
			if after+2 < 5 {
				next = fmt.Sprintf(`"%d"`, after+2)
			}
			io.WriteString(w, `{"data":`+numbers(after+1, after+2)+`,"meta":{"next":`+next+`}}`)
		case "/linked":
			page := number("page", 1)
			if page*2 < 5 {
				w.Header().Set("Link", fmt.Sprintf(`<%s/linked?page=%d>; rel="next"`, server.URL, page+1))
			}
			io.WriteString(w, numbers((page-1)*2+1, page*2))
		case "/broken":
			if number("page", 1) == 1 {
				io.WriteString(w, "[1,2]")
			} else {
				w.WriteHeader(http.StatusInternalServerError)
				io.WriteString(w, "down")
			}
		case "/endless":
			io.WriteString(w, fmt.Sprintf("[%d]", number("page", 1)))
		case "/hello":
			io.WriteString(w, "hello")
		case "/echo":
			body, _ := io.ReadAll(r.Body)
			io.WriteString(w, r.Method+" "+string(body))
		case "/header":
			io.WriteString(w, r.Header.Get("X-Test"))
		case "/query":
			io.WriteString(w, r.URL.RawQuery)
		case "/auth":
			if a := r.Header.Get("Authorization"); a != "" {
				io.WriteString(w, a)
			} else {
				io.WriteString(w, r.Header.Get("X-Api-Key"))
			}
		default:
			w.WriteHeader(http.StatusNotFound)
			io.WriteString(w, "no such page")
		}
	}))
	defer server.Close()
	for name, value := range file.Env {
		t.Setenv(name, value)
	}
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
		if c.Base != "" {
			var query []Header
			for _, q := range c.Query {
				query = append(query, Header{Name: q[0], Value: q[1]})
			}
			base := strings.ReplaceAll(c.Base, "{server}", server.URL)
			if c.Paging != nil {
				reply = HTTPPages(base, c.Path, query, headers, c.Auth, *c.Paging)
			} else {
				reply = HTTPAt(c.Method, base, c.Path, query, c.Body, headers, c.Auth)
			}
		}
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
