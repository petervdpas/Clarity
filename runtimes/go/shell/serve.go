// The web target (DESIGN 7.3): the application as a web server. It serves
// the renderer and its files to a browser, telling the page where to send
// its commands, and answers each command with the same Invoke the desktop
// shell binds into its window. Each browser tab runs the application of its
// own; the database is the server's.

package shell

import (
	"encoding/json"
	"fmt"
	"io/fs"
	"net/http"
	"os"
	"strings"
)

// InvokePath is where a page the server serves sends its commands.
const InvokePath = "/clarity/invoke"

// Handler serves assets, the page among them, and answers the page's commands.
func (c *Service) Handler(assets fs.FS) http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc(InvokePath, func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "send a command with POST", http.StatusMethodNotAllowed)
			return
		}
		var sent struct {
			Command string          `json:"command"`
			Args    json.RawMessage `json:"args"`
		}
		answer := Answer{}
		if err := json.NewDecoder(r.Body).Decode(&sent); err != nil {
			answer = Answer{Error: "a command is JSON with its command and its args: " + err.Error()}
		} else {
			answer = c.Invoke(r.Context(), sent.Command, sent.Args)
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(answer)
	})
	files := http.FileServerFS(assets)
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" && r.URL.Path != "/index.html" {
			files.ServeHTTP(w, r)
			return
		}
		page, err := fs.ReadFile(assets, "index.html")
		if err != nil {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		_, _ = w.Write([]byte(Served(string(page))))
	})
	return mux
}

// Served is the page as the server gives it: it says where its commands go.
func Served(page string) string {
	return strings.Replace(page, "</head>", fmt.Sprintf("<script>window.clarityServer = %q;</script></head>", InvokePath), 1)
}

// Serve runs the application as a web server on address, such as
// "127.0.0.1:8080", until it stops.
func Serve(options Options, address string) error {
	service := New(options)
	fmt.Fprintf(os.Stderr, "%s: serving on http://%s\n", options.Name, address)
	return http.ListenAndServe(address, service.Handler(options.Assets))
}

// Address is where the server listens: `--address host:port`, else the
// environment's CLARITY_ADDRESS, else project, where the project file says
// (its port).
func Address(args []string, project string) string {
	for i, arg := range args {
		if arg == "--address" && i+1 < len(args) {
			return args[i+1]
		}
	}
	if address := os.Getenv("CLARITY_ADDRESS"); address != "" {
		return address
	}
	return project
}
