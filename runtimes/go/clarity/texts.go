// Text functions in code (conformance/texts.json): characters counted once
// however many bytes they take, positions from 1 as lists count.

package clarity

import (
	"strings"
	"unicode/utf8"
)

// Words is words(text): the words of a text, apart by white space.
func TextWords(text string) []string {
	found := strings.Fields(text)
	if found == nil {
		return []string{}
	}
	return found
}

// Split is split(text, separator): the pieces between each separator, empty
// ones kept; an empty separator gives the characters.
func TextSplit(text, separator string) []string {
	return strings.Split(text, separator)
}

// Trim is trim(text): the text without white space at its ends.
func TextTrim(text string) string { return strings.TrimSpace(text) }

// Lower is lower(text): the text in lower case.
func TextLower(text string) string { return strings.ToLower(text) }

// Upper is upper(text): the text in upper case.
func TextUpper(text string) string { return strings.ToUpper(text) }

// Replace is replace(text, find, with): every find replaced; an empty find changes nothing.
func TextReplace(text, find, with string) string {
	if find == "" {
		return text
	}
	return strings.ReplaceAll(text, find, with)
}

// StartsWith is startsWith(text, prefix).
func TextStartsWith(text, prefix string) bool { return strings.HasPrefix(text, prefix) }

// EndsWith is endsWith(text, suffix).
func TextEndsWith(text, suffix string) bool { return strings.HasSuffix(text, suffix) }

// Slice is slice(text, from, count): count characters from position from
// (from 1), clamped to the text.
func TextSlice(text string, from, count int64) string {
	runes := []rune(text)
	start := max(from, 1)
	if count < 0 {
		count = 0
	}
	end := min(from+count, int64(len(runes))+1)
	if end <= start {
		return ""
	}
	return string(runes[start-1 : end-1])
}

// IndexOf is indexOf(text, find): the position of find's first character
// (from 1), 0 when it is not there.
func TextIndexOf(text, find string) int64 {
	at := strings.Index(text, find)
	if at < 0 {
		return 0
	}
	return int64(utf8.RuneCountInString(text[:at])) + 1
}
