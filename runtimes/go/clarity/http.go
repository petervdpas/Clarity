package clarity

// HTTP calls from code (DESIGN 5.23): `http get url` and the others,
// through net/http. A call never fails: when no answer comes, the reply has
// status 0 and says why, so the handler decides what that means. Every
// runtime gives the same replies (conformance/http.json).

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strconv"
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

// HTTP calls url with method (GET, POST, PUT, PATCH or DELETE), sending body
// with POST, PUT and PATCH and each of headers, and gives the reply.
func HTTP(method, url, body string, headers []Header) HttpReply {
	reply, _ := httpFetch(method, url, body, headers)
	return reply
}

// nextLink is the address a Link header names rel="next": <url>; rel="next".
func nextLink(header string) string {
	for _, part := range strings.Split(header, ",") {
		address, params, found := strings.Cut(part, ";")
		if !found {
			continue
		}
		for _, p := range strings.Split(params, ";") {
			p = strings.ReplaceAll(strings.TrimSpace(p), " ", "")
			if strings.EqualFold(p, `rel="next"`) || strings.EqualFold(p, "rel=next") {
				return strings.TrimSuffix(strings.TrimPrefix(strings.TrimSpace(address), "<"), ">")
			}
		}
	}
	return ""
}

// httpFetch is a call as HTTP makes it, with the address its Link header
// names next ("" without one).
func httpFetch(method, url, body string, headers []Header) (HttpReply, string) {
	scheme, _, _ := strings.Cut(url, "://")
	if !strings.Contains(url, "://") || (!strings.EqualFold(scheme, "http") && !strings.EqualFold(scheme, "https")) {
		return HttpReply{Text: "only http and https addresses are called, not " + url}, ""
	}
	var sent io.Reader
	if method == http.MethodPost || method == http.MethodPut || method == http.MethodPatch {
		sent = strings.NewReader(body)
	}
	request, err := http.NewRequest(method, url, sent)
	if err != nil {
		return HttpReply{Text: fmt.Sprintf("cannot reach %s: %v", HTTPHost(url), err)}, ""
	}
	for _, h := range headers {
		request.Header.Add(h.Name, h.Value)
	}
	response, err := httpClient.Do(request)
	if err != nil {
		return HttpReply{Text: fmt.Sprintf("cannot reach %s: %v", HTTPHost(url), err)}, ""
	}
	defer response.Body.Close()
	answer, err := io.ReadAll(response.Body)
	if err != nil {
		return HttpReply{Text: fmt.Sprintf("cannot read the answer of %s: %v", HTTPHost(url), err)}, ""
	}
	status := int64(response.StatusCode)
	next := ""
	for _, link := range response.Header.Values("Link") {
		if next = nextLink(link); next != "" {
			break
		}
	}
	return HttpReply{Status: status, Ok: status >= 200 && status < 300, Text: strings.ToValidUTF8(string(answer), "�")}, next
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
	reply, _ := httpFetchAt(method, base, path, query, body, headers, auth)
	return reply
}

// HTTPPaging is how a web API hands out a long list (DESIGN 5.23, paging):
// the project file's paging line. Kind is "page", "offset", "cursor" or
// "link"; Items is where a page's items are in its JSON, "" when a page is
// a list.
type HTTPPaging struct {
	Kind      string `json:"kind"`
	Param     string `json:"param"`
	SizeParam string `json:"size_param"`
	Size      int    `json:"size"`
	Next      string `json:"next"`
	Items     string `json:"items"`
}

// HTTPMostPages is how many pages are read at most.
const HTTPMostPages = 1000

// jsonAt is the JSON at a dotted path of value: meta.next; value itself for "".
func jsonAt(value json.RawMessage, path string) (json.RawMessage, bool) {
	if path == "" {
		return value, true
	}
	for _, step := range strings.Split(path, ".") {
		var object map[string]json.RawMessage
		if json.Unmarshal(value, &object) != nil {
			return nil, false
		}
		found, ok := object[step]
		if !ok {
			return nil, false
		}
		value = found
	}
	return value, true
}

// HTTPPages gets path of the web API at base page after page, as paging
// says, as `http get "/orders" from shop every page` does: one reply whose
// text is one JSON list of the items of every page (conformance/http.json).
func HTTPPages(base, path string, query []Header, headers []Header, auth HTTPAuth, paging HTTPPaging) HttpReply {
	var items []string
	cursor, link := "", ""
	for n := 0; n < HTTPMostPages; n++ {
		sent := append([]Header{}, query...)
		switch paging.Kind {
		case "page":
			sent = append(sent, Header{Name: paging.Param, Value: strconv.Itoa(n + 1)}, Header{Name: paging.SizeParam, Value: strconv.Itoa(paging.Size)})
		case "offset":
			sent = append(sent, Header{Name: paging.Param, Value: strconv.Itoa(n * paging.Size)}, Header{Name: paging.SizeParam, Value: strconv.Itoa(paging.Size)})
		case "cursor":
			if cursor != "" {
				sent = append(sent, Header{Name: paging.Param, Value: cursor})
			}
		}
		var reply HttpReply
		var next string
		if link != "" {
			// A next link is a whole address, with its own query.
			reply, next = httpFetchAt(http.MethodGet, "", link, nil, "", headers, auth)
		} else {
			reply, next = httpFetchAt(http.MethodGet, base, path, sent, "", headers, auth)
		}
		if !reply.Ok {
			return reply
		}
		var list []json.RawMessage
		found, ok := jsonAt(json.RawMessage(reply.Text), paging.Items)
		if !ok || json.Unmarshal(found, &list) != nil || bytes.HasPrefix(bytes.TrimSpace(found), []byte("null")) {
			place := paging.Items
			if place == "" {
				place = "the top"
			}
			return HttpReply{Text: fmt.Sprintf("page %d of %s has no list at %s", n+1, path, place)}
		}
		for _, item := range list {
			var compact bytes.Buffer
			json.Compact(&compact, item)
			items = append(items, compact.String())
		}
		done := false
		switch paging.Kind {
		case "page", "offset":
			done = len(list) < paging.Size
		case "cursor":
			cursor = ""
			if value, ok := jsonAt(json.RawMessage(reply.Text), paging.Next); ok {
				var text string
				var number json.Number
				if json.Unmarshal(value, &text) == nil {
					cursor = text
				} else if json.Unmarshal(value, &number) == nil {
					cursor = number.String()
				}
			}
			done = cursor == ""
		case "link":
			link = next
			done = link == ""
		}
		if done {
			return HttpReply{Status: reply.Status, Ok: true, Text: "[" + strings.Join(items, ",") + "]"}
		}
	}
	return HttpReply{Text: fmt.Sprintf("stopped after %d pages of %s", HTTPMostPages, path)}
}

// httpFetchAt is a call as HTTPAt makes it, with the address its Link
// header names next.
func httpFetchAt(method, base, path string, query []Header, body string, headers []Header, auth HTTPAuth) (HttpReply, string) {
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
	return httpFetch(method, url, body, all)
}
