// Every test of the runtime, in the order they run.

#pragma once

#define CLARITY_TESTS(X)                                          \
    X(a_decimal_keeps_its_decimals)                               \
    X(decimals_add_multiply_divide_and_compare_by_value)          \
    X(every_rounding_case_rounds_as_every_runtime_does)          \
    X(an_item_is_read_by_its_position_from_1)                    \
    X(an_item_is_set_by_its_position_from_1)                     \
    X(random_gives_what_every_runtime_gives_from_a_seed)         \
    X(random_stays_within_its_highest_number)                    \
    X(what_an_engine_keeps_outlives_the_scratch_arena)           \
    X(sin_and_cos_take_degrees_as_every_runtime_does)            \
    X(a_draw_starts_an_empty_drawing_that_travels_with_the_part)  \
    X(a_canvas_says_where_it_was_clicked_as_every_runtime_does)   \
    X(a_foldable_part_is_folded_as_every_runtime_says)           \
    X(a_sidebar_is_collapsed_and_maximized_as_every_runtime_says) \
    X(a_ribbons_open_tab_is_as_every_runtime_says)               \
    X(a_wizards_step_is_as_every_runtime_says)                   \
    X(an_accordions_open_section_is_as_every_runtime_says)       \
    X(a_custom_parts_values_and_hears_are_as_every_runtime_says) \
    X(dates_and_times_read_and_write_as_text)                     \
    X(the_settings_file_is_where_every_runtime_puts_it)           \
    X(settings_kept_are_read_back)                               \
    X(a_file_in_a_store_is_kept_under_its_name_in_a_folder_of_its_contents) \
    X(a_copy_goes_to_the_downloads_folder_under_a_free_name)     \
    X(a_file_is_kept_with_its_name_in_the_database_or_its_store) \
    X(a_list_is_kept_as_compact_json)                             \
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
    X(a_reply_carries_the_texts_shown_in_text_values)             \
    X(code_texts_are_translated_as_every_runtime_does)            \
    X(a_window_opens_with_its_data_and_with)                      \
    X(the_shell_answers_the_renderers_commands)                   \
    X(every_open_window_has_its_own_number)                       \
    X(a_closed_window_is_reported_to_what_listens)                \
    X(a_closed_window_is_reported)                                \
    X(show_and_text)                                              \
    X(files_are_filtered_and_sorted_by_their_details)            \
    X(bytes_are_read_and_written_as_they_are)                     \
    X(every_engine_call_gives_what_every_host_gives)              \
    X(a_module_that_is_no_engine_is_refused_when_it_loads)        \
    X(a_call_keeps_to_the_limits_of_its_engine)                   \
    X(an_engine_uses_the_services_it_declares)                    \
    X(an_engine_with_state_keeps_its_instance_from_call_to_call)  \
    X(an_engine_with_state_per_window_keeps_an_instance_for_each_window) \
    X(a_question_travels_in_the_reply_as_from_the_other_runtimes) \
    X(a_dialog_is_asked_for_and_closes_with_its_value)            \
    X(the_command_line_words_are_the_shared_ones)                 \
    X(command_lines_are_read_as_every_runtime_reads_them)         \
    X(a_printed_table_lines_up_as_every_runtime_does)             \
    X(a_command_asks_on_a_terminal_as_every_runtime_does)        \
    X(every_http_call_gives_the_reply_the_cases_name)            \
    X(a_url_names_its_host)                                      \
    X(json_is_fitted_as_every_runtime_fits_it)                    \
    X(a_secret_is_kept_read_and_forgotten)                       \
    X(settings_keep_their_secrets_out_of_the_file) \
    X(a_database_connects_with_a_secret_setting)

#define CLARITY_DECLARED(name) void name(void);
CLARITY_TESTS(CLARITY_DECLARED)
