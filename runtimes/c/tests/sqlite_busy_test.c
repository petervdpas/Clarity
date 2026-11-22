// Another program holding the database a moment (Clarity's data view,
// clarity-migrate, a second copy of the app) makes a write wait for it, up to
// five seconds, instead of failing with SQLITE_BUSY at once, as on every stack.

#include "check.h"
#include "clarity/db.h"
#include "tests.h"

void a_write_waits_while_another_program_holds_the_database(void) {
    const char* dir = clarity_format("%s/clarity-c-busy", temp_folder());
    remove_tree(dir);
    ClaritySource source = {"main", CLARITY_SQLITE, clarity_format("%s/shop.db", dir)};
    ClarityDb* db;
    REQUIRE_OK(clarity_db_open(&source, 1, &db));
    ClarityRows rows = {0};
    REQUIRE_OK(clarity_db_rows(db, "", "PRAGMA busy_timeout", NULL, &rows));
    REQUIRE(rows.count == 1);
    CHECK_INT((int)rows.items[0].values[0].integer, 5000);
    clarity_db_close(db);
    remove_tree(dir);
}
