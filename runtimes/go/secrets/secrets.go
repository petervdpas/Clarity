// Package secrets keeps the secret settings of a program (DESIGN 5.27) in
// the system's protected store: the Credential Manager on Windows, the
// Keychain on macOS, the Secret Service (GNOME Keyring, KWallet) elsewhere.
// A program with secret settings sets clarity.Secrets = secrets.System{}.
package secrets

import (
	"errors"
	"fmt"

	"github.com/zalando/go-keyring"
)

// System is the system's protected store: an entry is the program's name with the setting's.
type System struct{}

// Get gives the secret, or "" when it was never kept or the store cannot be reached.
func (System) Get(program, name string) string {
	secret, err := keyring.Get(program, name)
	if err != nil {
		return ""
	}
	return secret
}

// Set keeps the secret; "" forgets it.
func (System) Set(program, name, value string) error {
	var err error
	if value == "" {
		err = keyring.Delete(program, name)
		if errors.Is(err, keyring.ErrNotFound) {
			err = nil
		}
	} else {
		err = keyring.Set(program, name, value)
	}
	if err != nil {
		return fmt.Errorf("cannot keep the secret '%s' in the system's protected store: %w", name, err)
	}
	return nil
}
