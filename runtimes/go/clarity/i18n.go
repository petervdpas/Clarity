// Translation in the backend (DESIGN 1.5, I1c): text code makes, such as
// what a function returns, is translated in the language the screen picked
// before its slots are filled (conformance/translations.json, `code`). The
// translations come from the model the application is made with; the
// screen names the language with SetLanguage.

package clarity

import (
	"encoding/json"
	"sync"
)

var i18n struct {
	sync.RWMutex
	translations map[string]map[string]string
	language     *string
}

// LoadTranslations takes the translations from the model (project.translations).
func LoadTranslations(model []byte) {
	var m struct {
		Project struct {
			Translations map[string]map[string]string `json:"translations"`
		} `json:"project"`
	}
	_ = json.Unmarshal(model, &m)
	i18n.Lock()
	defer i18n.Unlock()
	i18n.translations = m.Project.Translations
}

// SetLanguage is the language texts show in, as the screen picked it; nil
// for the texts as written.
func SetLanguage(language *string) {
	i18n.Lock()
	defer i18n.Unlock()
	i18n.language = language
}

// Language is the language the screen picked; "" for the texts as written.
func Language() string {
	i18n.RLock()
	defer i18n.RUnlock()
	if i18n.language == nil {
		return ""
	}
	return *i18n.language
}

// Translate is text in the language picked, or as written when it has no
// translation there.
func Translate(text string) string {
	i18n.RLock()
	defer i18n.RUnlock()
	if i18n.language == nil {
		return text
	}
	if t, ok := i18n.translations[*i18n.language][text]; ok {
		return t
	}
	return text
}

// Translated is the same text in the language the screen picked, its values kept.
func (t Text) Translated() Text {
	t.Template = Translate(t.Template)
	return t
}
