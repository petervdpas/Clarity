package clarity

// HTTP calls from code (DESIGN 5.23): `http get url` and the others,
// through net/http. A call never fails: when no answer comes, the reply has
// status 0 and says why, so the handler decides what that means. Every
// runtime gives the same replies (conformance/http.json).

import (
	"encoding/base64"
	"fmt"
	"io"
	"net/http"
	"os"
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

// HTTPAuth is how a web API data source signs its calls: the project
// file's auth line, its secret read from where it says (env:NAME). Kind is
// "", "bearer", "api-key" or "basic".
type HTTPAuth struct {
	Kind   string `json:"kind"`
	Secret string `json:"secret"`
}

// secretAt is a secret where the project file says it is: env:NAME reads
// that environment variable, setting:<program>/<name> a secret setting in
// the system's protected store (DESIGN 5.27); empty when it is not there.
func secretAt(location string) string {
	if name, found := strings.CutPrefix(location, "env:"); found {
		return os.Getenv(name)
	}
	if at, found := strings.CutPrefix(location, "setting:"); found {
		return settingSecret(at)
	}
	return location
}

// URLPart is text as a part of a url: letters, digits and - . _ ~ as they
// are, every other byte of its UTF-8 as %XX.
func URLPart(text string) string {
	var out strings.Builder
	for _, b := range []byte(text) {
		switch {
		case b >= 'a' && b <= 'z', b >= 'A' && b <= 'Z', b >= '0' && b <= '9', b == '-', b == '.', b == '_', b == '~':
			out.WriteByte(b)
		default:
			fmt.Fprintf(&out, "%%%02X", b)
		}
	}
	return out.String()
}

// HTTPAt calls path of the web API at base (a data source's address) with
// the query parameters, encoded and in order, and the source's auth, as
// `http get "/orders" from shop query "status": s` does.
func HTTPAt(method, base, path string, query []Header, body string, headers []Header, auth HTTPAuth) HttpReply {
	url := path
	if base != "" && !strings.Contains(path, "://") {
		url = strings.TrimRight(base, "/") + "/" + strings.TrimLeft(path, "/")
	}
	for i, q := range query {
		joint := "&"
		if i == 0 && !strings.Contains(url, "?") {
			joint = "?"
		}
		url += joint + URLPart(q.Name) + "=" + URLPart(q.Value)
	}
	all := append([]Header{}, headers...)
	switch auth.Kind {
	case "bearer":
		all = append(all, Header{Name: "Authorization", Value: "Bearer " + secretAt(auth.Secret)})
	case "api-key":
		all = append(all, Header{Name: "X-Api-Key", Value: secretAt(auth.Secret)})
	case "basic":
		all = append(all, Header{Name: "Authorization", Value: "Basic " + base64.StdEncoding.EncodeToString([]byte(secretAt(auth.Secret)))})
	}
	return HTTP(method, url, body, all)
}
