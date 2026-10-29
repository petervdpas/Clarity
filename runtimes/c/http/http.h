// HTTP calls from code (DESIGN 5.23): `http get url` and the others, through
// libcurl. A call never fails: when no answer comes, the reply has status 0
// and says why, so the handler decides what that means. Every runtime gives
// the same replies (conformance/http.json). Only a project whose code calls
// out has this folder, so only it links libcurl.

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// The kind of call.
typedef enum {
    CLARITY_HTTP_GET,
    CLARITY_HTTP_POST,
    CLARITY_HTTP_PUT,
    CLARITY_HTTP_PATCH,
    CLARITY_HTTP_DELETE,
} ClarityHttpMethod;

/// A header a call sends.
typedef struct {
    const char* name;
    const char* value;
} ClarityHttpHeader;

/// What a web API answered: its status (0 when no answer came), whether it
/// is ok (200 to 299), and its text, or why no answer came. The text is in
/// the scratch arena.
typedef struct {
    int64_t status;
    bool ok;
    const char* text;
} ClarityHttpReply;

/// How many seconds a call waits for its whole answer.
#define CLARITY_HTTP_TIMEOUT 30L

/// Calls `url` with `method`, sending `body` with post and put (NULL sends
/// nothing) and the `count` headers of `headers`, and gives the reply.
ClarityHttpReply clarity_http_call(ClarityHttpMethod method, const char* url, const char* body, const ClarityHttpHeader* headers, size_t count);

/// How a web API data source signs its calls: the project file's `auth`
/// line, its secret read from where it says (`env:NAME`).
typedef enum {
    CLARITY_AUTH_NONE,
    CLARITY_AUTH_BEARER,
    CLARITY_AUTH_API_KEY,
    CLARITY_AUTH_BASIC,
} ClarityAuthKind;

/// Calls `path` of the web API at `base` (a data source's address) with the
/// `query_count` query parameters, encoded and in order, and the source's
/// auth, as `http get "/orders" from shop query "status": s` does.
ClarityHttpReply clarity_http_call_at(ClarityHttpMethod method, const char* base, const char* path, const ClarityHttpHeader* query, size_t query_count, const char* body,
                                      const ClarityHttpHeader* headers, size_t count, ClarityAuthKind auth, const char* secret);

/// How a web API hands out a long list (DESIGN 5.23, paging): the project
/// file's `paging` line.
typedef enum {
    CLARITY_PAGING_PAGE,
    CLARITY_PAGING_OFFSET,
    CLARITY_PAGING_CURSOR,
    CLARITY_PAGING_LINK,
} ClarityPagingKind;

/// A `paging` line: the query parameters it sends, the size of a page, where
/// the next cursor is and where a page's items are ("" when a page is a list).
typedef struct {
    ClarityPagingKind kind;
    const char* param;
    const char* size_param;
    int size;
    const char* next;
    const char* items;
} ClarityHttpPaging;

/// At most this many pages are read.
#define CLARITY_HTTP_MOST_PAGES 1000

/// Gets `path` of the web API at `base` page after page, as `paging` says, as
/// `http get "/orders" from shop every page` does: one reply whose text is
/// one JSON list of the items of every page (conformance/http.json).
ClarityHttpReply clarity_http_pages(const char* base, const char* path, const ClarityHttpHeader* query, size_t query_count, const ClarityHttpHeader* headers,
                                    size_t count, ClarityAuthKind auth, const char* secret, ClarityHttpPaging paging);

/// Text as a part of a url: letters, digits and `- . _ ~` as they are, every
/// other byte of its UTF-8 as `%XX`; in the scratch arena.
const char* clarity_url_part(const char* text);

/// The host a url names, without the scheme, a login, the port and the
/// path, in the scratch arena.
const char* clarity_http_host(const char* url);

#ifdef __cplusplus
}
#endif
