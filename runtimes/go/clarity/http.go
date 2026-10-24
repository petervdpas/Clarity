package clarity

// HTTP calls from code (DESIGN 5.23): `http get url` and the others,
// through net/http. A call never fails: when no answer comes, the reply has
// status 0 and says why, so the handler decides what that means. Every
// runtime gives the same replies (conformance/http.json).

import (
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// HTTPTimeout is how long a call waits for its whole answer.
const HTTPTimeout = 30 * time.Second

// HttpReply is what a web API answered: its status (0 when no answer came),
// whether it is ok (200 to 299), and its text, or why no answer came.
type HttpReply struct {
	Status int64
	Ok     bool
	Text   string
}

// Header is a header a call sends.
type Header struct {
	Name  string
	Value string
}

var httpClient = &http.Client{Timeout: HTTPTimeout}

// HTTPHost is the host a url names, without the scheme, a login, the port
// and the path, for saying it cannot be reached.
func HTTPHost(url string) string {
	rest := url
	if _, after, found := strings.Cut(url, "://"); found {
		rest = after
	}
	if at := strings.IndexAny(rest, "/?#"); at >= 0 {
		rest = rest[:at]
	}
	if at := strings.LastIndex(rest, "@"); at >= 0 {
		rest = rest[at+1:]
	}
	if strings.HasPrefix(rest, "[") {
		if end := strings.Index(rest, "]"); end >= 0 {
			return rest[1:end]
		}
		return rest[1:]
	}
	host, _, _ := strings.Cut(rest, ":")
	return host
}

// HTTP calls url with method (GET, POST, PUT or DELETE), sending body with
// POST and PUT and each of headers, and gives the reply.
func HTTP(method, url, body string, headers []Header) HttpReply {
	scheme, _, _ := strings.Cut(url, "://")
	if !strings.Contains(url, "://") || (!strings.EqualFold(scheme, "http") && !strings.EqualFold(scheme, "https")) {
		return HttpReply{Text: "only http and https addresses are called, not " + url}
	}
	var sent io.Reader
	if method == http.MethodPost || method == http.MethodPut {
		sent = strings.NewReader(body)
	}
	request, err := http.NewRequest(method, url, sent)
	if err != nil {
		return HttpReply{Text: fmt.Sprintf("cannot reach %s: %v", HTTPHost(url), err)}
	}
	for _, h := range headers {
		request.Header.Add(h.Name, h.Value)
	}
	response, err := httpClient.Do(request)
	if err != nil {
		return HttpReply{Text: fmt.Sprintf("cannot reach %s: %v", HTTPHost(url), err)}
	}
	defer response.Body.Close()
	answer, err := io.ReadAll(response.Body)
	if err != nil {
		return HttpReply{Text: fmt.Sprintf("cannot read the answer of %s: %v", HTTPHost(url), err)}
	}
	status := int64(response.StatusCode)
	return HttpReply{Status: status, Ok: status >= 200 && status < 300, Text: strings.ToValidUTF8(string(answer), "�")}
}
