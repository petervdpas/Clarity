package clarity

// Secret settings (DESIGN 5.27): kept in the system's protected store, per
// person and per application, never in settings.json. The store is the
// package secrets of this module, which a program with secret settings sets
// as Secrets; the others never load it.

import (
	"strings"
	"sync"
)

// SecretStore keeps a program's secrets: Get gives "" for none, Set with ""
// forgets it.
type SecretStore interface {
	Get(program, name string) string
	Set(program, name, value string) error
}

// Secrets is the store of this program's secret settings; nil when it has none.
var Secrets SecretStore

// keptSecrets is each secret as the store holds it now, as read or kept, by
// program and name; none is "".
var keptSecrets sync.Map

// ReadSettingsWith is ReadSettings with the secrets named, each from the
// store, beside the file's values.
func ReadSettingsWith(program string, secrets []string) map[string]any {
	values := ReadSettings(program)
	for _, name := range secrets {
		delete(values, name)
		if Secrets == nil {
			continue
		}
		if secret := Secrets.Get(program, name); secret != "" {
			values[name] = secret
			keptSecrets.Store(program+"\x00"+name, secret)
		}
	}
	return values
}

// KeepSettingsWith is KeepSettings with the secrets named kept in the store
// and never in the file.
func KeepSettingsWith(program string, values map[string]any, secrets []string) error {
	plain := make(map[string]any, len(values))
	for name, value := range values {
		plain[name] = value
	}
	for _, name := range secrets {
		value, found := plain[name]
		if !found {
			continue
		}
		delete(plain, name)
		if Secrets == nil {
			continue
		}
		// Only a secret that changed goes to the store: another setting
		// changing never needs it, nor fails where it cannot be reached.
		text, _ := value.(string)
		key := program + "\x00" + name
		before, _ := keptSecrets.Load(key)
		if was, _ := before.(string); was == text {
			continue
		}
		if err := Secrets.Set(program, name, text); err != nil {
			return err
		}
		keptSecrets.Store(key, text)
	}
	return KeepSettings(program, plain)
}

// settingSecret is the secret a web API's auth names as
// `setting:<program>/<name>`, from the store; "" without one.
func settingSecret(at string) string {
	program, name, found := strings.Cut(at, "/")
	if !found || Secrets == nil {
		return ""
	}
	return Secrets.Get(program, name)
}
