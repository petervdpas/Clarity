// The values records hold: exact decimals, dates and times, and text with
// slots as it travels to the screen.

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

static ClarityDecimal d(const char* text) {
    ClarityDecimal out = {0, 0};
    clarity_decimal_parse(text, &out);
    return out;
}

void a_decimal_keeps_its_decimals(void) {
    CHECK_TEXT(clarity_decimal_text(d("1250.50")), "1250.50");
    CHECK_TEXT(clarity_decimal_text(d("-0.05")), "-0.05");
    CHECK_TEXT(clarity_decimal_text(d("7")), "7");
    CHECK_TEXT(clarity_decimal_text(clarity_decimal_of(42)), "42");
    CHECK_TEXT(clarity_decimal_text(clarity_decimal(125050, 2)), "1250.50");
    ClarityDecimal out;
    CHECK_FAILS(CLARITY_INVALID, clarity_decimal_parse("12,5", &out));
    CHECK_FAILS(CLARITY_INVALID, clarity_decimal_parse("", &out));
}

void decimals_add_multiply_divide_and_compare_by_value(void) {
    CHECK_TEXT(clarity_decimal_text(clarity_decimal_add(d("1.25"), d("2.5"))), "3.75");
    CHECK_TEXT(clarity_decimal_text(clarity_decimal_sub(d("1.25"), clarity_decimal_of(2))), "-0.75");
    CHECK_TEXT(clarity_decimal_text(clarity_decimal_mul(d("19.99"), clarity_decimal_of(3))), "59.97");
    ClarityDecimal q;
    CHECK_OK(clarity_decimal_div(clarity_decimal_of(10), clarity_decimal_of(4), &q));
    CHECK_TEXT(clarity_decimal_text(q), "2.5");
    CHECK_OK(clarity_decimal_div(clarity_decimal_of(1), clarity_decimal_of(3), &q));
    CHECK_TEXT(clarity_decimal_text(q), "0.3333333333333333");
    CHECK_OK(clarity_decimal_div(clarity_decimal_of(2), clarity_decimal_of(3), &q));
    CHECK_TEXT(clarity_decimal_text(q), "0.6666666666666667");
    CHECK_FAILS(CLARITY_INVALID, clarity_decimal_div(clarity_decimal_of(1), clarity_decimal_of(0), &q));
    CHECK(clarity_decimal_compare(d("1.50"), d("1.5")) == 0);
    CHECK(clarity_decimal_compare(d("99.5"), d("1250.50")) < 0);
    CHECK_TEXT(clarity_decimal_text(clarity_decimal_rescaled(d("2.345"), 2)), "2.35");
    CHECK_TEXT(clarity_decimal_text(clarity_decimal_rescaled(d("-2.345"), 2)), "-2.35");
    // Money as SQL Server and the other databases keep it.
    CHECK_TEXT(clarity_decimal_text(d("922337203685477.5807")), "922337203685477.5807");
}

void dates_and_times_read_and_write_as_text(void) {
    ClarityDate date = {0}, other = {0};
    ClarityDateTime time = {0};
    CHECK(clarity_date_parse("2026-10-17", &date));
    CHECK_TEXT(clarity_date_text(date), "2026-10-17");
    CHECK(!clarity_date_parse("2026-02-30", &other));
    CHECK(!clarity_date_parse("17-10-2026", &other));
    CHECK(clarity_date_time_parse("2026-10-17T09:30:15", &time));
    CHECK_TEXT(clarity_date_time_text(time), "2026-10-17T09:30:15");
    CHECK(clarity_date_time_parse("2026-10-17 09:30", &time));
    CHECK_TEXT(clarity_date_time_text(time), "2026-10-17T09:30:00");
    CHECK(!clarity_date_time_parse("2026-10-17T25:00:00", &time));
    CHECK(clarity_date_parse("2026-01-02", &other));
    CHECK(clarity_date_compare(other, date) < 0);
    CHECK_INT(clarity_date_compare((ClarityDate){0}, other), -1);
    CHECK_TEXT(clarity_date_text((ClarityDate){0}), "");
    CHECK(clarity_today().year >= 2026);
}

void text_travels_as_the_other_runtimes_send_it(void) {
    CHECK_JSON(clarity_text_json(clarity_plain_text("Brace { yourself }")), "\"Brace { yourself }\"");
    ClarityDate since;
    clarity_date_parse("2026-10-17", &since);
    ClarityText* saved = clarity_text("Saved {customer.name} on {when}: {total}, {paid}, {kind}");
    clarity_text_with(saved, "customer.name", clarity_slot_text("Ada"));
    clarity_text_with(saved, "when", clarity_slot_date(since, "long"));
    clarity_text_with(saved, "total", clarity_slot_decimal(d("1250.50"), 2, NULL));
    clarity_text_with(saved, "paid", clarity_slot_bool(true));
    clarity_text_with(saved, "kind", clarity_slot_enum("Business", "CustomerKind"));
    CHECK_JSON(clarity_text_json(saved),
               "{\"text\":\"Saved {customer.name} on {when}: {total}, {paid}, {kind}\",\"values\":{\"customer.name\":{\"kind\":\"text\",\"value\":\"Ada\"},"
               "\"kind\":{\"kind\":\"enum\",\"value\":\"Business\",\"enum\":\"CustomerKind\"},\"paid\":{\"kind\":\"bool\",\"value\":true},"
               "\"total\":{\"kind\":\"decimal\",\"value\":\"1250.50\",\"scale\":2},\"when\":{\"kind\":\"date\",\"value\":\"2026-10-17\",\"format\":\"long\"}}}");
    CHECK_TEXT(clarity_text_show(saved), "Saved Ada on 2026-10-17: 1250.50, yes, Business");
    ClarityText* back;
    REQUIRE_OK(clarity_text_from_json(clarity_text_json(saved), &back));
    CHECK_TEXT(clarity_text_show(back), clarity_text_show(saved));
    CHECK_TEXT(clarity_text_show(clarity_text("{{braces}} and }")), "{braces} and }");
    // A key travels with its slot values; the screen looks up its value.
    ClarityText* keyed = clarity_text_with(clarity_key_text("orderApproved"), "number", clarity_slot_text("A-7"));
    CHECK_JSON(clarity_text_json(keyed), "{\"key\":\"orderApproved\",\"values\":{\"number\":{\"kind\":\"text\",\"value\":\"A-7\"}}}");
    CHECK_JSON(clarity_text_json(clarity_key_text("save")), "{\"key\":\"save\"}");
    ClarityText* keyed_back;
    REQUIRE_OK(clarity_text_from_json(clarity_text_json(keyed), &keyed_back));
    CHECK_TEXT(keyed_back->key, "orderApproved");
}

void the_scratch_arena_is_emptied_after_a_command(void) {
    clarity_scratch_reset();
    for (int i = 0; i < 1000; ++i) clarity_format("value %d of a command that holds a lot of text", i);
    CHECK(clarity_scratch_used() > 10000);
    clarity_scratch_reset();
    CHECK_INT(clarity_scratch_used(), 0);
}

void show_and_text(void) {
    CHECK_FAILS(CLARITY_CONTRACT, clarity_require(false, "order.total > 0"));
    CHECK_TEXT(clarity_error()->message, "condition failed: order.total > 0");
    CHECK(clarity_require(true, "fine"));
    CHECK_TEXT(clarity_show_integer(42), "42");
    CHECK_TEXT(clarity_show_bool(true), "yes");
    CHECK_TEXT(clarity_show_decimal(d("9.95")), "9.95");
    ClarityDateTime t;
    clarity_date_time_parse("2026-10-17T09:30:15", &t);
    CHECK_TEXT(clarity_show_date_time(t), "2026-10-17 09:30");
    CHECK_TEXT(clarity_show_date((ClarityDate){0}), "");
    CHECK_TEXT(clarity_join("Order ", "7"), "Order 7");
    CHECK(clarity_same_text(NULL, ""));
    CHECK(clarity_text_contains("Ada Lovelace", "LOVE"));
    CHECK(!clarity_text_contains("Ada", "Grace"));
}

void every_rounding_case_rounds_as_every_runtime_does(void) {
    cJSON* file = conformance("rounding.json");
    const cJSON* cases = cJSON_GetObjectItemCaseSensitive(file, "cases");
    REQUIRE(cJSON_GetArraySize(cases) > 0);
    const cJSON* c;
    cJSON_ArrayForEach(c, cases) {
        const char* value = cJSON_GetObjectItemCaseSensitive(c, "value")->valuestring;
        int places = cJSON_GetObjectItemCaseSensitive(c, "places")->valueint;
        ClarityDecimal gives = d(cJSON_GetObjectItemCaseSensitive(c, "gives")->valuestring);
        ClarityDecimal rounded = clarity_decimal_round(d(value), places);
        CHECK(clarity_decimal_compare(rounded, gives) == 0);
    }
    cJSON_Delete(file);
}

void sin_and_cos_take_degrees_as_every_runtime_does(void) {
    cJSON* file = conformance("drawing.json");
    const cJSON* angles = cJSON_GetObjectItemCaseSensitive(file, "angles");
    REQUIRE(cJSON_GetArraySize(angles) > 0);
    const cJSON* c;
    cJSON_ArrayForEach(c, angles) {
        ClarityDecimal degrees = d(cJSON_GetObjectItemCaseSensitive(c, "degrees")->valuestring);
        CHECK(clarity_decimal_compare(clarity_decimal_sin(degrees), d(cJSON_GetObjectItemCaseSensitive(c, "sin")->valuestring)) == 0);
        CHECK(clarity_decimal_compare(clarity_decimal_cos(degrees), d(cJSON_GetObjectItemCaseSensitive(c, "cos")->valuestring)) == 0);
    }
    cJSON_Delete(file);
}

void a_canvas_says_where_it_was_clicked_as_every_runtime_does(void) {
    cJSON* file = conformance("drawing.json");
    const cJSON* points = cJSON_GetObjectItemCaseSensitive(file, "points");
    REQUIRE(cJSON_GetArraySize(points) > 0);
    const cJSON* c;
    cJSON_ArrayForEach(c, points) {
        cJSON* in = cJSON_CreateObject();
        cJSON_AddStringToObject(in, "window", "Main");
        cJSON_AddItemToObject(in, "parts", cJSON_CreateObject());
        cJSON_AddItemToObject(cJSON_GetObjectItemCaseSensitive(in, "parts"), "board",
                              cJSON_Duplicate(cJSON_GetObjectItemCaseSensitive(c, "part"), true));
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(in, &state));
        ClarityUi* ui = clarity_ui(&state);
        CHECK(clarity_ui_x(ui, "board") == (int64_t)cJSON_GetObjectItemCaseSensitive(c, "x")->valuedouble);
        CHECK(clarity_ui_y(ui, "board") == (int64_t)cJSON_GetObjectItemCaseSensitive(c, "y")->valuedouble);
        CHECK(clarity_ui_from_x(ui, "board") == (int64_t)cJSON_GetObjectItemCaseSensitive(c, "fromX")->valuedouble);
        CHECK(clarity_ui_from_y(ui, "board") == (int64_t)cJSON_GetObjectItemCaseSensitive(c, "fromY")->valuedouble);
        CHECK_TEXT(clarity_ui_bent(ui, "board"), cJSON_GetObjectItemCaseSensitive(c, "bent")->valuestring);
        const cJSON* want = cJSON_GetObjectItemCaseSensitive(c, "via");
        ClarityIntegers via = clarity_ui_via(ui, "board");
        CHECK_INT(via.count, cJSON_GetArraySize(want));
        for (size_t i = 0; i < via.count; ++i) CHECK_INT(via.items[i], (int64_t)cJSON_GetArrayItem(want, (int)i)->valuedouble);
        // What the pointer holds besides x and y travels back unchanged.
        cJSON* back = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
        CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(back, "parts"), "board"),
                   cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "part")));
    }
    cJSON_Delete(file);
}

void a_foldable_part_is_folded_as_every_runtime_says(void) {
    cJSON* file = conformance("folding.json");
    const cJSON* c;
    REQUIRE(cJSON_GetArraySize(cJSON_GetObjectItemCaseSensitive(file, "cases")) > 0);
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "cases")) {
        cJSON* in = cJSON_CreateObject();
        cJSON_AddStringToObject(in, "window", "Main");
        cJSON_AddItemToObject(in, "parts", cJSON_CreateObject());
        cJSON_AddItemToObject(cJSON_GetObjectItemCaseSensitive(in, "parts"), "details", cJSON_Duplicate(cJSON_GetObjectItemCaseSensitive(c, "part"), true));
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(in, &state));
        ClarityUi* ui = clarity_ui(&state);
        CHECK(clarity_ui_folded(ui, "details", cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(c, "design"))) == cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(c, "folded")));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "set")) {
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(cJSON_Parse("{\"window\":\"Main\",\"data\":{}}"), &state));
        ClarityUi* ui = clarity_ui(&state);
        clarity_ui_set_folded(ui, "details", cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(c, "folded")));
        cJSON* back = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
        CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(back, "parts"), "details"),
                   cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "part")));
    }
    cJSON_Delete(file);
}

static ClarityUi* sidebar_ui(const cJSON* part) {
    cJSON* in = cJSON_CreateObject();
    cJSON_AddStringToObject(in, "window", "Main");
    cJSON_AddItemToObject(in, "parts", cJSON_CreateObject());
    cJSON_AddItemToObject(cJSON_GetObjectItemCaseSensitive(in, "parts"), "nav", cJSON_Duplicate(part, true));
    ClarityWindowState* state = clarity_alloc(sizeof *state);
    if (!clarity_window_state_from_json(in, state)) return NULL;
    return clarity_ui(state);
}

void a_sidebar_is_collapsed_and_maximized_as_every_runtime_says(void) {
    cJSON* file = conformance("sidebars.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "collapsed")) {
        ClarityUi* ui = sidebar_ui(cJSON_GetObjectItemCaseSensitive(c, "part"));
        REQUIRE(ui);
        CHECK(clarity_ui_collapsed(ui, "nav", cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(c, "design"))) == cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(c, "collapsed")));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "maximized")) {
        ClarityUi* ui = sidebar_ui(cJSON_GetObjectItemCaseSensitive(c, "part"));
        REQUIRE(ui);
        CHECK(clarity_ui_maximized(ui, "nav") == cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(c, "maximized")));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "set")) {
        ClarityUi* ui = sidebar_ui(cJSON_CreateObject());
        REQUIRE(ui);
        const cJSON* collapsed = cJSON_GetObjectItemCaseSensitive(c, "collapsed");
        const cJSON* maximized = cJSON_GetObjectItemCaseSensitive(c, "maximized");
        if (cJSON_IsBool(collapsed)) clarity_ui_set_collapsed(ui, "nav", cJSON_IsTrue(collapsed));
        if (cJSON_IsBool(maximized)) clarity_ui_set_maximized(ui, "nav", cJSON_IsTrue(maximized));
        cJSON* back = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
        CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(back, "parts"), "nav"),
                   cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "part")));
    }
    cJSON_Delete(file);
}

void a_wizards_step_is_as_every_runtime_says(void) {
    cJSON* file = conformance("steps.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "step")) {
        cJSON* in = cJSON_CreateObject();
        cJSON_AddStringToObject(in, "window", "Main");
        cJSON_AddItemToObject(in, "parts", cJSON_CreateObject());
        cJSON_AddItemToObject(cJSON_GetObjectItemCaseSensitive(in, "parts"), "wizard", cJSON_Duplicate(cJSON_GetObjectItemCaseSensitive(c, "part"), true));
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(in, &state));
        CHECK(clarity_ui_step(clarity_ui(&state), "wizard") == (int64_t)cJSON_GetObjectItemCaseSensitive(c, "step")->valuedouble);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "set")) {
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(cJSON_Parse("{\"window\":\"Main\",\"data\":{}}"), &state));
        ClarityUi* ui = clarity_ui(&state);
        clarity_ui_set_step(ui, "wizard", (int64_t)cJSON_GetObjectItemCaseSensitive(c, "step")->valuedouble);
        cJSON* back = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
        CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(back, "parts"), "wizard"),
                   cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "part")));
    }
    cJSON_Delete(file);
}

void an_accordions_open_section_is_as_every_runtime_says(void) {
    cJSON* file = conformance("accordions.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "open")) {
        cJSON* in = cJSON_CreateObject();
        cJSON_AddStringToObject(in, "window", "Main");
        cJSON_AddItemToObject(in, "parts", cJSON_CreateObject());
        cJSON_AddItemToObject(cJSON_GetObjectItemCaseSensitive(in, "parts"), "faq", cJSON_Duplicate(cJSON_GetObjectItemCaseSensitive(c, "part"), true));
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(in, &state));
        CHECK(clarity_ui_open_section(clarity_ui(&state), "faq") == (int64_t)cJSON_GetObjectItemCaseSensitive(c, "open")->valuedouble);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "set")) {
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(cJSON_Parse("{\"window\":\"Main\",\"data\":{}}"), &state));
        ClarityUi* ui = clarity_ui(&state);
        clarity_ui_set_open_section(ui, "faq", (int64_t)cJSON_GetObjectItemCaseSensitive(c, "open")->valuedouble);
        cJSON* back = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
        CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(back, "parts"), "faq"),
                   cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "part")));
    }
    cJSON_Delete(file);
}

void a_ribbons_open_tab_is_as_every_runtime_says(void) {
    cJSON* file = conformance("ribbons.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "tab")) {
        cJSON* in = cJSON_CreateObject();
        cJSON_AddStringToObject(in, "window", "Main");
        cJSON_AddItemToObject(in, "parts", cJSON_CreateObject());
        cJSON_AddItemToObject(cJSON_GetObjectItemCaseSensitive(in, "parts"), "mainRibbon", cJSON_Duplicate(cJSON_GetObjectItemCaseSensitive(c, "part"), true));
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(in, &state));
        CHECK(clarity_ui_tab(clarity_ui(&state), "mainRibbon") == (int64_t)cJSON_GetObjectItemCaseSensitive(c, "tab")->valuedouble);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "set")) {
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(cJSON_Parse("{\"window\":\"Main\",\"data\":{}}"), &state));
        ClarityUi* ui = clarity_ui(&state);
        clarity_ui_set_tab(ui, "mainRibbon", (int64_t)cJSON_GetObjectItemCaseSensitive(c, "tab")->valuedouble);
        cJSON* back = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
        CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(back, "parts"), "mainRibbon"),
                   cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "part")));
    }
    cJSON_Delete(file);
}

void a_draw_starts_an_empty_drawing_that_travels_with_the_part(void) {
    ClarityWindowState state;
    REQUIRE_OK(clarity_window_state_from_json(cJSON_Parse("{\"window\":\"Main\",\"data\":{}}"), &state));
    ClarityUi* ui = clarity_ui(&state);
    clarity_ui_begin_drawing(ui, "dial");
    clarity_ui_draw(ui, "dial", cJSON_Parse("{\"op\":\"clear\",\"color\":\"#101810\"}"));
    cJSON* circle = cJSON_CreateObject();
    cJSON_AddStringToObject(circle, "op", "circle");
    cJSON_AddNumberToObject(circle, "r", clarity_units(d("40.5")));
    clarity_ui_draw(ui, "dial", circle);
    CHECK_JSON(clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state),
               "{\"window\":\"Main\",\"instance\":0,\"data\":{},\"parts\":{\"dial\":{\"drawing\":[{\"op\":\"clear\",\"color\":\"#101810\"},{\"op\":\"circle\",\"r\":40.5}]}},\"selected\":{}}");
    // The next draw starts anew.
    ClarityUi* again = clarity_ui(&state);
    clarity_ui_begin_drawing(again, "dial");
    CHECK_JSON(clarity_window_state_json(&clarity_ui_reply(again, "Main", cJSON_CreateObject())->state),
               "{\"window\":\"Main\",\"instance\":0,\"data\":{},\"parts\":{\"dial\":{\"drawing\":[]}},\"selected\":{}}");
}

void an_item_is_read_by_its_position_from_1(void) {
    cJSON* file = conformance("lists.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "items")) {
        const cJSON* values = cJSON_GetObjectItemCaseSensitive(c, "list");
        int64_t items[16];
        size_t count = 0;
        const cJSON* v;
        cJSON_ArrayForEach(v, values) items[count++] = (int64_t)v->valuedouble;
        ClarityIntegers list = clarity_integers(count, items);
        int64_t position = (int64_t)cJSON_GetObjectItemCaseSensitive(c, "at")->valuedouble;
        int64_t got = 0;
        const cJSON* gives = cJSON_GetObjectItemCaseSensitive(c, "gives");
        if (gives) {
            CHECK_OK(clarity_integers_at(list, position, &got));
            CHECK_INT(got, (int64_t)gives->valuedouble);
        } else {
            CHECK_FAILS(CLARITY_VALUE, clarity_integers_at(list, position, &got));
            CHECK_TEXT(clarity_error()->message, cJSON_GetObjectItemCaseSensitive(c, "fails")->valuestring);
        }
    }
    cJSON_Delete(file);
}

void an_item_is_set_by_its_position_from_1(void) {
    ClarityIntegers list = clarity_integers(3, (int64_t[]){10, 20, 30});
    // A copy has items of its own.
    ClarityIntegers copy = clarity_integers(list.count, list.items);
    CHECK_OK(clarity_integers_set(list, 2, 99));
    CHECK_INT(list.items[1], 99);
    CHECK_INT(copy.items[1], 20);
    CHECK_FAILS(CLARITY_VALUE, clarity_integers_set(list, 4, 1));
    CHECK_TEXT(clarity_error()->message, "there is no item 4 in a list of 3");
}

void a_custom_parts_values_and_hears_are_as_every_runtime_says(void) {
    cJSON* file = conformance("custom-parts.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "read")) {
        cJSON* in = cJSON_CreateObject();
        cJSON_AddStringToObject(in, "window", "Main");
        cJSON_AddItemToObject(in, "parts", cJSON_CreateObject());
        cJSON_AddItemToObject(cJSON_GetObjectItemCaseSensitive(in, "parts"), "arena", cJSON_Duplicate(cJSON_GetObjectItemCaseSensitive(c, "part"), true));
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(in, &state));
        const cJSON* read = clarity_ui_part_value(clarity_ui(&state), "arena", cJSON_GetObjectItemCaseSensitive(c, "name")->valuestring);
        cJSON* got = read ? cJSON_Duplicate(read, true) : cJSON_CreateNull();
        CHECK_JSON(got, cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "value")));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "set")) {
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(cJSON_Parse("{\"window\":\"Main\",\"data\":{}}"), &state));
        ClarityUi* ui = clarity_ui(&state);
        const cJSON* set;
        cJSON_ArrayForEach(set, cJSON_GetObjectItemCaseSensitive(c, "sets")) {
            clarity_ui_set_part_value(ui, "arena", cJSON_GetArrayItem(set, 0)->valuestring, cJSON_Duplicate(cJSON_GetArrayItem(set, 1), true));
        }
        cJSON* back = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
        CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(back, "parts"), "arena"),
                   cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "part")));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "hear")) {
        ClarityWindowState state;
        REQUIRE_OK(clarity_window_state_from_json(cJSON_Parse("{\"window\":\"Main\",\"data\":{}}"), &state));
        ClarityUi* ui = clarity_ui(&state);
        const cJSON* hear;
        cJSON_ArrayForEach(hear, cJSON_GetObjectItemCaseSensitive(c, "hears")) {
            clarity_ui_hear(ui, cJSON_GetArrayItem(hear, 0)->valuestring, cJSON_GetArrayItem(hear, 1)->valuestring, cJSON_Duplicate(cJSON_GetArrayItem(hear, 2), true));
        }
        cJSON* reply = clarity_reply_json(clarity_ui_reply(ui, "Main", cJSON_CreateObject()));
        const cJSON* hears = cJSON_GetObjectItemCaseSensitive(reply, "hears");
        CHECK_JSON(hears ? cJSON_Duplicate(hears, true) : cJSON_CreateNull(), cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(c, "reply")));
    }
    cJSON_Delete(file);
}

void ranges_and_joins_are_as_every_runtime_gives_them(void) {
    cJSON* file = conformance("lists.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "ranges")) {
        ClarityIntegers got = clarity_range((int64_t)cJSON_GetObjectItem(c, "from")->valuedouble, (int64_t)cJSON_GetObjectItem(c, "to")->valuedouble);
        const cJSON* gives = cJSON_GetObjectItem(c, "gives");
        CHECK(got.count == (size_t)cJSON_GetArraySize(gives));
        for (size_t i = 0; i < got.count && i < (size_t)cJSON_GetArraySize(gives); ++i) CHECK(got.items[i] == (int64_t)cJSON_GetArrayItem(gives, (int)i)->valuedouble);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItemCaseSensitive(file, "joins")) {
        const cJSON* texts = cJSON_GetObjectItem(c, "texts");
        size_t count = (size_t)cJSON_GetArraySize(texts);
        const char** items = clarity_alloc(sizeof *items * (count + 1));
        for (size_t i = 0; i < count; ++i) items[i] = cJSON_GetArrayItem(texts, (int)i)->valuestring;
        CHECK_TEXT(clarity_joined(clarity_texts(count, items), cJSON_GetObjectItem(c, "separator")->valuestring), cJSON_GetObjectItem(c, "gives")->valuestring);
    }
}

// `snapped(value, step)`: the multiple of step nearest to value, as a canvas's grid snaps (conformance/drawing.json, grids).
void snapped_rounds_to_its_step_as_every_runtime_does(void) {
    cJSON* file = conformance("drawing.json");
    const cJSON* cases = cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(file, "grids"), "snapped");
    REQUIRE(cJSON_GetArraySize(cases) > 0);
    const cJSON* c;
    cJSON_ArrayForEach(c, cases) {
        int64_t value = (int64_t)cJSON_GetObjectItemCaseSensitive(c, "value")->valuedouble;
        int64_t step = (int64_t)cJSON_GetObjectItemCaseSensitive(c, "step")->valuedouble;
        CHECK_INT(clarity_snapped(value, step), (int64_t)cJSON_GetObjectItemCaseSensitive(c, "result")->valuedouble);
    }
    cJSON_Delete(file);
}

// A canvas's grid and snap are as its line says until code sets them.
void a_canvas_keeps_the_grid_and_snap_code_sets(void) {
    ClarityWindowState state = {0};
    ClarityUi* ui = clarity_ui(&state);
    CHECK_INT(clarity_ui_grid(ui, "sheet", 20), 20);
    CHECK(clarity_ui_snap(ui, "sheet", true));
    clarity_ui_set_grid(ui, "sheet", 0);
    clarity_ui_set_snap(ui, "sheet", false);
    CHECK_INT(clarity_ui_grid(ui, "sheet", 20), 0);
    CHECK(!clarity_ui_snap(ui, "sheet", true));
    cJSON* out = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
    CHECK_JSON(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(out, "parts"), "sheet"), "{\"grid\":0,\"snap\":false}");
    CHECK_TEXT(clarity_ui_background(ui, "sheet", "panel"), "panel");
    clarity_ui_set_background(ui, "sheet", "#f5f0e1");
    CHECK_TEXT(clarity_ui_background(ui, "sheet", "panel"), "#f5f0e1");
    clarity_ui_set_dot_size(ui, "sheet", 3);
    clarity_ui_set_dot_color(ui, "sheet", "muted");
    CHECK_INT(clarity_ui_dot_size(ui, "sheet", 2), 3);
    CHECK_TEXT(clarity_ui_dot_color(ui, "sheet", ""), "muted");
    // A painting or vector canvas's tool, brush and ink (DESIGN 5.31), and the chosen shape from 1.
    CHECK_TEXT(clarity_ui_tool(ui, "sheet", "pen"), "pen");
    CHECK_INT(clarity_ui_brush(ui, "sheet", 2), 2);
    CHECK_TEXT(clarity_ui_ink(ui, "sheet", "text"), "text");
    clarity_ui_set_tool(ui, "sheet", "eraser");
    clarity_ui_set_brush(ui, "sheet", 6);
    clarity_ui_set_ink(ui, "sheet", "#ff0000");
    CHECK_TEXT(clarity_ui_tool(ui, "sheet", "pen"), "eraser");
    CHECK_INT(clarity_ui_brush(ui, "sheet", 2), 6);
    CHECK_TEXT(clarity_ui_ink(ui, "sheet", "text"), "#ff0000");
    cJSON* kept = clarity_window_state_json(&clarity_ui_reply(ui, "Main", cJSON_CreateObject())->state);
    const cJSON* sheet = cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(kept, "parts"), "sheet");
    CHECK_JSON(cJSON_GetObjectItemCaseSensitive(sheet, "tool"), "\"eraser\"");
    CHECK_JSON(cJSON_GetObjectItemCaseSensitive(sheet, "brush"), "6");
    CHECK_JSON(cJSON_GetObjectItemCaseSensitive(sheet, "ink"), "\"#ff0000\"");
    CHECK_INT(clarity_ui_selected_shape(ui, "drawing"), 0);
    ClaritySelected chosen = {"drawing", 2};
    ClarityWindowState drawn = {.selected = &chosen, .selected_count = 1};
    CHECK_INT(clarity_ui_selected_shape(clarity_ui(&drawn), "drawing"), 3);
}

// `copy text`: the reply hands the text to the screen for its clipboard; of several, the last.
void a_handler_copies_the_last_text_it_copies(void) {
    ClarityWindowState state = {0};
    ClarityUi* ui = clarity_ui(&state);
    clarity_ui_copy(ui, clarity_plain_text("first"));
    clarity_ui_copy(ui, clarity_plain_text("classDiagram"));
    cJSON* reply = clarity_reply_json(clarity_ui_reply(ui, "Main", cJSON_CreateObject()));
    CHECK_JSON(cJSON_GetObjectItemCaseSensitive(reply, "copy"), "\"classDiagram\"");
    ClarityUi* quiet = clarity_ui(&state);
    CHECK(cJSON_GetObjectItemCaseSensitive(clarity_reply_json(clarity_ui_reply(quiet, "Main", cJSON_CreateObject())), "copy") == NULL);
}
