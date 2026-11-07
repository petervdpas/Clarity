#pragma once

// The text functions of code (conformance/texts.json): characters counted
// once however many bytes they take in UTF-8, positions from 1.

#include <cstdint>
#include <string>
#include <vector>

namespace clarity {

/// `words(text)`: the words of a text, apart by white space.
std::vector<std::string> words(const std::string& text);
/// `split(text, separator)`: the pieces between each separator, empty ones kept; an empty separator gives the characters.
std::vector<std::string> split(const std::string& text, const std::string& separator);
/// `trim(text)`: the text without white space at its ends.
std::string trim(const std::string& text);
/// `lower(text)` and `upper(text)`: each letter to one letter of the other case (ß stays ß).
std::string lower(const std::string& text);
std::string upper(const std::string& text);
/// `replace(text, find, with)`: every find replaced; an empty find changes nothing.
std::string replace(const std::string& text, const std::string& find, const std::string& with);
bool starts_with(const std::string& text, const std::string& prefix);
bool ends_with(const std::string& text, const std::string& suffix);
/// `slice(text, from, count)`: count characters from position from (from 1), clamped to the text.
std::string slice(const std::string& text, int64_t from, int64_t count);
/// `indexOf(text, find)`: the position of find's first character (from 1), 0 when it is not there.
int64_t index_of(const std::string& text, const std::string& find);
/// `zip(paths, texts)` (conformance/zip.json): a zip archive of one file per path holding the text of
/// the same place, stored as it is, in order; the lists pair up. The same files give the same bytes.
std::vector<uint8_t> zip(const std::vector<std::string>& paths, const std::vector<std::string>& texts);

}  // namespace clarity
