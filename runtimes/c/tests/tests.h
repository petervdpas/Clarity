// Every test of the runtime, in the order they run.

#pragma once

#define CLARITY_TESTS(X)                                          \
    X(a_decimal_keeps_its_decimals)                               \
    X(decimals_add_multiply_divide_and_compare_by_value)          \
    X(every_rounding_case_rounds_as_every_runtime_does)          \
    X(sin_and_cos_take_degrees_as_every_runtime_does)            \
    X(a_draw_starts_an_empty_drawing_that_travels_with_the_part)  \
    X(dates_and_times_read_and_write_as_text)                     \
    X(text_travels_as_the_other_runtimes_send_it)                 \
    X(the_scratch_arena_is_emptied_after_a_command)               \
    X(every_query_case_gives_its_sql_on_every_database)           \
    X(a_picture_is_told_by_its_first_bytes_and_named_by_them)     \
    X(a_picture_travels_to_the_renderer_as_every_runtime_sends_it) \
    X(a_picture_is_kept_in_the_database_and_fetched_on_its_own)   \
    X(a_picture_in_a_file_store_is_a_file_named_by_its_bytes)     \
    X(a_store_lists_its_files_as_every_runtime_does)              \
    X(events_run_around_a_save)                                   \
    X(a_failing_event_keeps_nothing)                              \
    X(events_run_around_a_delete)                                 \
    X(the_server_scenario_on_sqlite)                              \
    X(a_handler_replies_as_the_other_runtimes_do)                 \
    X(rows_read_again_have_no_selected_row)                       \
    X(a_window_opens_with_its_data_and_with)                      \
    X(the_shell_answers_the_renderers_commands)                   \
    X(show_and_text)                                              \
    X(files_are_filtered_and_sorted_by_their_details)            \
    X(every_engine_call_gives_what_every_host_gives)              \
    X(a_module_that_is_no_engine_is_refused_when_it_loads)        \
    X(a_call_keeps_to_the_limits_of_its_engine)                   \
    X(an_engine_uses_the_services_it_declares)

#define CLARITY_DECLARED(name) void name(void);
CLARITY_TESTS(CLARITY_DECLARED)
