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
