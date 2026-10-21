// Texts by key in the backend (DESIGN 1.5, I1c): a text code makes, such as
// what a function returns, takes the value of its key in the language the
// screen picked, or the default language's when that language leaves it
// empty (conformance/translations.json, `code`). The values come from the
// model the application is made with; the screen names the language with
// SetLanguage.

package clarity

import (
	"encoding/json"
	"sync"
)

var i18n struct {
	sync.RWMutex
	values   map[string]map[string]string
	fallback string
	picked   *string
}

// LoadTranslations takes every language's values from the model
// (project.translations), and the default language (project.locale.language).
func LoadTranslations(model []byte) {
	var m struct {
		Project struct {
			Translations map[string]map[string]string `json:"translations"`
			Locale       struct {
				Language *string `json:"language"`
			} `json:"locale"`
		} `json:"project"`
	}
	_ = json.Unmarshal(model, &m)
	i18n.Lock()
	defer i18n.Unlock()
	i18n.values = m.Project.Translations
	i18n.fallback = "en"
	if m.Project.Locale.Language != nil {
		i18n.fallback = *m.Project.Locale.Language
	}
}

// SetLanguage is the language texts show in, as the screen picked it; nil
// for the default.
func SetLanguage(language *string) {
	i18n.Lock()
	defer i18n.Unlock()
	i18n.picked = language
}

// Language is the language the screen picked; "" for the default.
func Language() string {
	i18n.RLock()
	defer i18n.RUnlock()
	if i18n.picked == nil {
		return ""
	}
	return *i18n.picked
}

// KeyValue is the value of key in the language picked, or the default
// language's when that one leaves it empty; ok is false when no language
// has the key.
func KeyValue(key string) (string, bool) {
	i18n.RLock()
	defer i18n.RUnlock()
	of := func(language string) (string, bool) {
		v, ok := i18n.values[language][key]
		return v, ok && v != ""
	}
	if i18n.picked != nil {
		if v, ok := of(*i18n.picked); ok {
			return v, true
		}
	}
	return of(i18n.fallback)
}

// TextOf is the value of the key key_or_text names, or the text itself when
// it names none: what an engine gives.
func TextOf(keyOrText string) string {
	if v, ok := KeyValue(keyOrText); ok {
		return v
	}
	return keyOrText
}
