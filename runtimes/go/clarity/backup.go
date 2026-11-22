package clarity

// A backup of an SQLite database before migration steps change it
// (docs/ledger/backups.md): a copy beside it in backups/, made with VACUUM
// INTO, named <database>-<when>-before-migration-NNNN.db with the time in
// UTC, as Clarity and every runtime name it (conformance/backups.json). The
// newest ten automatic backups are kept; a manual one is the person's and
// stays.

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"
)

// KeptAutomatic is how many automatic backups each database keeps.
const KeptAutomatic = 10

// BackupStamp is a moment, in seconds since 1970, as a backup names it: 20261105T205905Z.
func BackupStamp(seconds int64) string {
	return time.Unix(seconds, 0).UTC().Format("20060102T150405Z")
}

// BackupFileName is the file of a backup of the database named database (its
// file's name without .db) taken at seconds before step.
func BackupFileName(database string, seconds int64, step int) string {
	return fmt.Sprintf("%s-%s-before-migration-%04d.db", database, BackupStamp(seconds), step)
}

type backupFile struct {
	when  string
	count int
	path  string
}

// parseBackup reads a backup file of the database named database: its stamp,
// its count within that second, and whether it is automatic.
func parseBackup(database, file string) (backupFile, bool, bool) {
	rest, ok := strings.CutPrefix(file, database+"-")
	if !ok {
		return backupFile{}, false, false
	}
	rest, ok = strings.CutSuffix(rest, ".db")
	if !ok {
		return backupFile{}, false, false
	}
	when, why, ok := strings.Cut(rest, "-")
	if !ok || len(when) != 16 || !strings.HasSuffix(when, "Z") {
		return backupFile{}, false, false
	}
	count := 1
	if head, n, found := strings.Cut(why, "_"); found {
		c, err := strconv.Atoi(n)
		if err != nil {
			return backupFile{}, false, false
		}
		why, count = head, c
	}
	return backupFile{when: when, count: count}, why != "manual", true
}

// PruneBackups removes the automatic backups of database beyond the newest
// ten; manual ones, other databases' and other files stay.
func PruneBackups(database string) {
	name := strings.TrimSuffix(filepath.Base(database), filepath.Ext(database))
	dir := filepath.Join(filepath.Dir(database), "backups")
	entries, err := os.ReadDir(dir)
	if err != nil {
		return
	}
	var automatic []backupFile
	for _, e := range entries {
		b, auto, ok := parseBackup(name, e.Name())
		if ok && auto {
			b.path = filepath.Join(dir, e.Name())
			automatic = append(automatic, b)
		}
	}
	sort.Slice(automatic, func(i, j int) bool {
		if automatic[i].when != automatic[j].when {
			return automatic[i].when > automatic[j].when
		}
		return automatic[i].count > automatic[j].count
	})
	for i := KeptAutomatic; i < len(automatic); i++ {
		_ = os.Remove(automatic[i].path)
	}
}
