// The binding between a window on the screen and its handlers: the state
// the screen sends, and the reply it gets, in the same JSON as from the
// other runtimes.

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

void a_handler_replies_as_the_other_runtimes_do(void) {
    ClarityWindowState state;
    REQUIRE_OK(clarity_window_state_from_json(
        cJSON_Parse("{\"window\":\"Card\",\"data\":{\"order\":{\"id\":7,\"number\":\"ORD-7\"},\"orders\":[{\"id\":1,\"number\":\"A\"},{\"id\":2,\"number\":\"B\"}]},"
                    "\"parts\":{\"name_filter\":{\"text\":\"Ada\"},\"mine\":{\"checked\":true}},\"selected\":{\"grid\":1}}"),
        &state));
    ClarityUi* ui = clarity_ui(&state);
    CHECK_TEXT(clarity_ui_text(ui, "name_filter"), "Ada");
    CHECK_TEXT(clarity_ui_text(ui, "other"), "");
    CHECK(clarity_ui_checked(ui, "mine"));
    CHECK(clarity_ui_visible(ui, "save"));
    CHECK(clarity_ui_readonly(ui, "name", true));
    CHECK_TEXT(clarity_ui_caption(ui, "save", "Save"), "Save");
    size_t row;
    CHECK_OK(clarity_ui_selected(ui, "grid", 2, &row));
    CHECK_INT(row, 1);
    CHECK_FAILS(CLARITY_INVALID, clarity_ui_selected(ui, "other", 2, &row));
    CHECK_FAILS(CLARITY_INVALID, clarity_ui_selected(ui, "grid", 1, &row));

    clarity_ui_set_enabled(ui, "save", false);
    clarity_ui_set_error(ui, "name", clarity_plain_text("Name is required"));
    clarity_ui_set_readonly(ui, "name", false);
    clarity_ui_set_text(ui, "name_filter", "");
    ClarityText* saved = clarity_text_with(clarity_text("Saved {n}"), "n", clarity_slot_text("ORD-7"));
    clarity_ui_message(ui, CLARITY_INFO, saved);
    clarity_ui_open(ui, "Order", clarity_json_object(1, "order", cJSON_Parse("{\"id\":7,\"number\":\"ORD-7\"}")));
    clarity_ui_close(ui);
    cJSON* data = cJSON_Duplicate(state.data, true);
    cJSON_ReplaceItemInObject(cJSON_GetObjectItem(data, "order"), "number", cJSON_CreateString("ORD-8"));
    CHECK_JSON(clarity_reply_json(clarity_ui_reply(ui, "Card", data)),
               "{\"state\":{\"window\":\"Card\",\"data\":{\"order\":{\"id\":7,\"number\":\"ORD-8\"},\"orders\":[{\"id\":1,\"number\":\"A\"},{\"id\":2,\"number\":\"B\"}]},"
               "\"parts\":{\"mine\":{\"checked\":true},\"name\":{\"error\":\"Name is required\",\"readonly\":false},\"name_filter\":{\"text\":\"\"},"
               "\"save\":{\"enabled\":false}},\"selected\":{\"grid\":1}},\"messages\":[{\"level\":\"info\",\"text\":{\"text\":\"Saved {n}\","
               "\"values\":{\"n\":{\"kind\":\"text\",\"value\":\"ORD-7\"}}}}],\"open\":[{\"window\":\"Order\",\"with\":{\"order\":{\"id\":7,\"number\":\"ORD-7\"}}}],"
               "\"close\":true}");
    // The handler worked on a copy: the state it came from keeps its parts.
    CHECK_TEXT(state.parts[0].text, "Ada");
    // A new window's state has no parts and no selected rows yet.
    ClarityWindowState fresh = {.window = "Card", .data = cJSON_CreateObject()};
    CHECK_JSON(clarity_window_state_json(&fresh), "{\"data\":{},\"parts\":{},\"selected\":{},\"window\":\"Card\"}");
}

void a_window_opens_with_its_data_and_with(void) {
    ClarityWindowState* state =
        clarity_opened("Card", cJSON_Parse("{\"order\":{\"id\":0,\"number\":\"new\"},\"note\":\"\"}"), cJSON_Parse("{\"order\":{\"id\":7,\"number\":\"ORD-7\"}}"));
    CHECK_JSON(state->data, "{\"order\":{\"id\":7,\"number\":\"ORD-7\"},\"note\":\"\"}");
    ClarityDecimal d;
    clarity_decimal_parse("12.50", &d);
    CHECK_JSON(clarity_json_decimal(d), "\"12.50\"");
    cJSON* record = cJSON_Parse("{\"total\":\"12.5\",\"count\":3,\"since\":\"2026-10-17\",\"none\":null,\"bad\":\"17-10-2026\",\"customer\":null}");
    ClarityDecimal total = {0, 0};
    CHECK_OK(clarity_json_get_decimal(record, "total", &total));
    CHECK(clarity_decimal_compare(total, d) == 0);
    int64_t count = 0, customer = 5;
    CHECK_OK(clarity_json_get_integer(record, "count", &count));
    CHECK_INT(count, 3);
    // A missing or null field keeps what the record had.
    CHECK_OK(clarity_json_get_reference(record, "customer", &customer));
    CHECK_INT(customer, 5);
    ClarityDate since = {0};
    CHECK_OK(clarity_json_get_date(record, "since", &since));
    CHECK_TEXT(clarity_date_text(since), "2026-10-17");
    CHECK_FAILS(CLARITY_INVALID, clarity_json_get_date(record, "bad", &since));
    CHECK_JSON(clarity_json_date((ClarityDate){0}), "null");
    CHECK_JSON(clarity_json_reference(0), "null");
}
