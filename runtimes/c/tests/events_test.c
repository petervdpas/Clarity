// Data events: a record whose entity has them runs them around its save and
// delete, in one transaction with them, as on the other stacks.

#include <ctype.h>
#include <stddef.h>

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

typedef struct {
    int64_t id;
    const char* text;
} LogLine;

static const char* const log_columns[] = {"text"};

static void log_values(const void* record, ClarityValue* out) { out[0] = clarity_text_value(((const LogLine*)record)->text); }

static bool log_from_values(int64_t id, const ClarityValue* v, void* record) {
    LogLine* r = record;
    r->id = id;
    return clarity_get_text("text", v[0], &r->text);
}

static const ClarityEntity log_line = {
    .name = "LogLine", .table = "log", .key = "id", .source = "", .columns = log_columns, .column_count = 1,
    .size = sizeof(LogLine), .id_at = offsetof(LogLine, id), .values = log_values, .from_values = log_from_values,
};

typedef struct {
    int64_t id;
    const char* name;
    int64_t balance;
} Account;

static const char* const account_columns[] = {"name", "balance"};

static void account_values(const void* record, ClarityValue* out) {
    const Account* r = record;
    out[0] = clarity_text_value(r->name);
    out[1] = clarity_integer_value(r->balance);
}

static bool account_from_values(int64_t id, const ClarityValue* v, void* record) {
    Account* r = record;
    r->id = id;
    return clarity_get_text("name", v[0], &r->name) && clarity_get_integer("balance", v[1], &r->balance);
}

static bool logged_line(ClarityDb* db, const char* text) {
    LogLine line = {0, text};
    return clarity_save(db, &log_line, &line);
}

static bool before_save(ClarityDb* db, void* record) {
    (void)db;
    Account* a = record;
    const char* start = a->name;
    while (*start == ' ') ++start;
    if (!*start) return clarity_refuse(clarity_plain_text("An account needs a name"));
    size_t length = strlen(start);
    while (length && start[length - 1] == ' ') --length;
    a->name = clarity_copy_n(start, length);
    return true;
}

static bool after_save(ClarityDb* db, void* record) {
    Account* a = record;
    if (!logged_line(db, clarity_format("saved %s with %lld", a->name, (long long)a->balance))) return false;
    if (a->balance < 0) return clarity_refuse(clarity_plain_text("An account cannot be in debt"));
    return true;
}

static bool before_delete(ClarityDb* db, void* record) {
    (void)db;
    Account* a = record;
    if (a->balance == 0) return true;
    ClarityText* t = clarity_text("{name} still holds {balance}");
    clarity_text_with(t, "name", clarity_slot_text(a->name));
    clarity_text_with(t, "balance", clarity_slot_integer(a->balance, NULL));
    return clarity_refuse(t);
}

static bool after_delete(ClarityDb* db, void* record) { return logged_line(db, clarity_format("deleted %s", ((Account*)record)->name)); }

static const ClarityEntity account = {
    .name = "Account", .table = "account", .key = "id", .source = "", .columns = account_columns, .column_count = 2,
    .size = sizeof(Account), .id_at = offsetof(Account, id), .values = account_values, .from_values = account_from_values,
    .before_save = before_save, .after_save = after_save, .before_delete = before_delete, .after_delete = after_delete,
};

static bool accounts(ClarityDb** out) {
    static const char* const create[] = {"CREATE TABLE account (id INTEGER PRIMARY KEY, name TEXT NOT NULL, balance INTEGER NOT NULL)",
                                         "CREATE TABLE log (id INTEGER PRIMARY KEY, text TEXT NOT NULL)"};
    ClarityStep step = {create, 2};
    return clarity_db_memory(out) && clarity_migrate(*out, "", &step, 1, NULL);
}

/// The log, one line after another.
static const char* logged(ClarityDb* db) {
    void* items;
    size_t count;
    if (!clarity_find(db, &log_line, clarity_query(), &items, &count)) return "(cannot read the log)";
    ClarityBuffer out = {0};
    for (size_t i = 0; i < count; ++i) clarity_buffer_addf(&out, "%s%s", i ? "; " : "", ((LogLine*)items)[i].text);
    return clarity_buffer_text(&out);
}

static size_t stored(ClarityDb* db) {
    void* items;
    size_t count = 0;
    clarity_find(db, &account, clarity_query(), &items, &count);
    return count;
}

/// What the person reads of the refusal of a call that failed.
static const char* refusal(bool worked) {
    if (worked) return "(not refused)";
    return clarity_error()->kind == CLARITY_REFUSED ? clarity_text_show(clarity_error()->text) : clarity_error()->message;
}

void events_run_around_a_save(void) {
    ClarityDb* db;
    REQUIRE_OK(accounts(&db));
    Account ada = {0, "  Ada ", 10};
    CHECK_OK(clarity_save(db, &account, &ada));
    // Before save changed the record that was stored.
    CHECK_TEXT(ada.name, "Ada");
    Account read;
    CHECK_OK(clarity_get(db, &account, ada.id, &read));
    CHECK_TEXT(read.name, "Ada");
    CHECK_TEXT(logged(db), "saved Ada with 10");
    Account nameless = {0, "", 0};
    CHECK_TEXT(refusal(clarity_save(db, &account, &nameless)), "An account needs a name");
    CHECK_INT(stored(db), 1);
    clarity_db_close(db);
}

void a_failing_event_keeps_nothing(void) {
    ClarityDb* db;
    REQUIRE_OK(accounts(&db));
    Account debtor = {0, "Grace", -5};
    CHECK_TEXT(refusal(clarity_save(db, &account, &debtor)), "An account cannot be in debt");
    // The record, and what after save wrote, are gone; the record is new again.
    CHECK_INT(debtor.id, 0);
    CHECK_INT(stored(db), 0);
    CHECK_TEXT(logged(db), "");
    // The database is usable afterwards.
    debtor.balance = 0;
    CHECK_OK(clarity_save(db, &account, &debtor));
    CHECK_TEXT(logged(db), "saved Grace with 0");
    clarity_db_close(db);
}

void events_run_around_a_delete(void) {
    ClarityDb* db;
    REQUIRE_OK(accounts(&db));
    Account ada = {0, "Ada", 10};
    CHECK_OK(clarity_save(db, &account, &ada));
    CHECK_TEXT(refusal(clarity_delete(db, &account, &ada)), "Ada still holds 10");
    ada.balance = 0;
    CHECK_OK(clarity_save(db, &account, &ada));
    CHECK_OK(clarity_delete(db, &account, &ada));
    CHECK_INT(stored(db), 0);
    CHECK_TEXT(logged(db), "saved Ada with 10; saved Ada with 0; deleted Ada");
    clarity_db_close(db);
}
