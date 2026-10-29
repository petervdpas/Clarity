// HTTP calls from code (DESIGN 5.23): `http get url` and the others, through
// libcurl. A call never fails: when no answer comes, the reply has status 0
// and says why, so the handler decides what that means. Every runtime gives
// the same replies (conformance/http.json). Only a project whose code calls
// out has this folder, so only it links libcurl.

#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace clarity {

/// The kind of call.
enum class HttpMethod { Get, Post, Put, Patch, Delete };

/// What a web API answered: its status (0 when no answer came), whether it
/// is ok (200 to 299), and its text, or why no answer came.
struct HttpReply {
    int64_t status = 0;
    bool ok = false;
    std::string text;
};

/// How many seconds a call waits for its whole answer.
inline constexpr long HTTP_TIMEOUT = 30;

/// Calls `url` with `method`, sending `body` with post and put and each of
/// `headers` (name, value), and gives the reply.
HttpReply http_call(HttpMethod method, const std::string& url, const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers);

/// How a web API data source signs its calls: the project file's `auth`
/// line, its secret read from where it says (`env:NAME`).
enum class HttpAuth { None, Bearer, ApiKey, Basic };

/// Calls `path` of the web API at `base` (a data source's address) with the
/// query parameters, encoded and in order, and the source's auth, as
/// `http get "/orders" from shop query "status": s` does.
HttpReply http_call_at(HttpMethod method, const std::string& base, const std::string& path, const std::vector<std::pair<std::string, std::string>>& query,
                       const std::string& body, const std::vector<std::pair<std::string, std::string>>& headers, HttpAuth auth, const std::string& secret);

/// How a web API hands out a long list (DESIGN 5.23, paging): the project
/// file's `paging` line.
enum class PagingKind { Page, Offset, Cursor, Link };

/// A `paging` line: the query parameters it sends, the size of a page, where
/// the next cursor is and where a page's items are (empty when a page is a list).
struct HttpPaging {
    PagingKind kind = PagingKind::Page;
    std::string param;
    std::string size_param;
    int size = 0;
    std::string next;
    std::string items;
};

/// At most this many pages are read.
inline constexpr int HTTP_MOST_PAGES = 1000;

/// Gets `path` of the web API at `base` page after page, as `paging` says, as
/// `http get "/orders" from shop every page` does: one reply whose text is
/// one JSON list of the items of every page (conformance/http.json).
HttpReply http_pages(const std::string& base, const std::string& path, const std::vector<std::pair<std::string, std::string>>& query,
                     const std::vector<std::pair<std::string, std::string>>& headers, HttpAuth auth, const std::string& secret, const HttpPaging& paging);

/// Text as a part of a url: letters, digits and `- . _ ~` as they are, every
/// other byte of its UTF-8 as `%XX`.
std::string url_part(const std::string& text);

/// The host a url names, without the scheme, a login, the port and the path.
std::string http_host(const std::string& url);

}  // namespace clarity
