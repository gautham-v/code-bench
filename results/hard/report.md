# hard: 44 runs

Cost is the CLI's list-price `total_cost_usd`. A run passes when it meets every check of its task; score is the share of checks met. Tasks marked private come from repos that aren't public, so their prompts, tests and diffs aren't in this repo; their numbers are.

## Per config

| config | runs | passed | checks met | mean score | said done but failed | timeouts | wall, mean | wall, median | cost, mean | tool calls, median | output tokens, median | lines +/−, median |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| sonnet medium | 4 | 0 | 24/40 | 0.600 | 4 | 0 | 3.2 min | 2.9 min | $0.52 | 16 | 19836 | +276 / −6 |
| sonnet high | 8 | 0 | 67/80 | 0.838 | 8 | 0 | 6.4 min | 6.7 min | $1.16 | 27 | 46149 | +509 / −14 |
| sonnet xhigh | 4 | 1 | 37/40 | 0.925 | 3 | 0 | 22.6 min | 16.5 min | $3.64 | 57 | 118486 | +999 / −29 |
| opus medium | 8 | 2 | 69/80 | 0.863 | 6 | 0 | 12.5 min | 10.2 min | $3.14 | 24 | 56006 | +841 / −23 |
| opus high | 8 | 5 | 76/80 | 0.950 | 3 | 0 | 22.5 min | 20.7 min | $5.76 | 46 | 112259 | +1028 / −35 |
| fable low | 8 | 3 | 73/80 | 0.912 | 5 | 0 | 8.4 min | 7.3 min | $4.88 | 19 | 39722 | +611 / −20 |
| fable medium | 4 | 1 | 36/40 | 0.900 | 3 | 0 | 14.0 min | 14.1 min | $7.71 | 26 | 74869 | +887 / −29 |

## Score by task and round

| task | sonnet medium | sonnet high | sonnet xhigh | opus medium | opus high | fable low | fable medium |
|---|---|---|---|---|---|---|---|
| catcher-big-feature | 0.6 | 0.9 / 0.8 | 1 | 0.6 / 0.8 | 1 / 0.8 | 0.9 / 1 | 0.8 |
| catcher-deep-bug | 0.5 | 0.9 / 0.9 | 0.9 | 1 / 1 | 0.9 / 1 | 1 / 1 | 0.9 |
| sidecar-hard (private) | 0.7 | 0.8 / 0.7 | 0.9 | 0.9 / 0.8 | 1 / 1 | 0.8 / 0.8 | 1 |
| squawk-hard | 0.6 | 0.9 / 0.8 | 0.9 | 0.9 / 0.9 | 1 / 0.9 | 0.9 / 0.9 | 0.9 |

## Wall time by task and round

| task | sonnet medium | sonnet high | sonnet xhigh | opus medium | opus high | fable low | fable medium |
|---|---|---|---|---|---|---|---|
| catcher-big-feature | 4.2 min | 8.7 min / 7.8 min | 19.4 min | 9.8 min / 9.9 min | 18.8 min / 15.3 min | 9.0 min / 7.6 min | 10.6 min |
| catcher-deep-bug | 88 s | 2.6 min / 3.4 min | 9.1 min | 8.3 min / 8.5 min | 18.5 min / 12.2 min | 4.4 min / 6.0 min | 7.2 min |
| sidecar-hard (private) | 92 s | 4.1 min / 5.5 min | 13.6 min | 10.5 min / 11.8 min | 24.0 min / 22.5 min | 6.9 min / 6.0 min | 17.7 min |
| squawk-hard | 5.7 min | 9.2 min / 9.8 min | 48.4 min | 21.2 min / 19.7 min | 36.4 min / 32.0 min | 14.4 min / 12.7 min | 20.6 min |

## Checks, by how many runs met them

| task | check | sonnet medium | sonnet high | sonnet xhigh | opus medium | opus high | fable low | fable medium |
|---|---|---|---|---|---|---|---|---|
| catcher-big-feature | existing_tests_pass | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-big-feature | plain_editing_unchanged | 1/1 | 1/2 | 1/1 | 2/2 | 2/2 | 2/2 | 0/1 |
| catcher-big-feature | same_number_lists_left_alone | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-big-feature | paste_joins_count | 0/1 | 2/2 | 1/1 | 0/2 | 1/2 | 2/2 | 1/1 |
| catcher-big-feature | nested_lists_count_separately | 0/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-big-feature | code_and_other_lists_untouched | 1/1 | 2/2 | 1/1 | 1/2 | 2/2 | 2/2 | 1/1 |
| catcher-big-feature | enter_delete_move_recount | 0/1 | 1/2 | 1/1 | 0/2 | 1/2 | 2/2 | 1/1 |
| catcher-big-feature | tab_nests_shift_tab_unnests | 0/1 | 1/2 | 1/1 | 1/2 | 2/2 | 1/2 | 0/1 |
| catcher-big-feature | edit_is_one_undo_step | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-big-feature | cursor_stays_on_its_text | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | existing_tests_pass | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | folds_follow_outside_edit | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | cursor_on_screen_after_outside_edit | 0/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | folds_follow_renamed_or_moved_file | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | reading_view_has_the_folds | 0/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | search_hit_in_fold_is_shown | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | search_hit_matches_unsaved_buffer | 0/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | search_hit_gone_keeps_cursor | 0/1 | 0/2 | 0/1 | 2/2 | 1/2 | 2/2 | 0/1 |
| catcher-deep-bug | heading_link_is_not_a_tag | 0/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| catcher-deep-bug | env_dir_not_written_to_settings | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| sidecar-hard (private) | existing_tests_pass | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| sidecar-hard (private) | auto_cue_basics_hold | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| sidecar-hard (private) | card_in_use_is_kept | 0/1 | 0/2 | 1/1 | 2/2 | 2/2 | 1/2 | 1/1 |
| sidecar-hard (private) | unfinished_ask_not_cued | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| sidecar-hard (private) | acknowledgement_not_cued | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| sidecar-hard (private) | fire_dropped_when_room_goes_on | 0/1 | 1/2 | 1/1 | 1/2 | 2/2 | 1/2 | 1/1 |
| sidecar-hard (private) | queued_fire_not_released_on_half_ask | 0/1 | 0/2 | 0/1 | 0/2 | 2/2 | 0/2 | 1/1 |
| sidecar-hard (private) | talked_over_cue_redone_for_whole_ask | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| sidecar-hard (private) | hotkey_routes_on_new_words | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| sidecar-hard (private) | answered_card_dims | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | existing_tests_pass | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | spoken_file_names_become_mentions | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | dotted_names_taken_whole | 0/1 | 0/2 | 0/1 | 0/2 | 1/2 | 0/2 | 0/1 |
| squawk-hard | spoken_folder_has_to_match | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | unsure_references_stay_as_spoken | 0/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | protected_text_left_alone | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | repo_terms_spelled_like_the_repo | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | vocab_lists_the_repos_files | 1/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | vocab_cache_follows_the_repo | 0/1 | 2/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |
| squawk-hard | finds_the_agent_session | 0/1 | 1/2 | 1/1 | 2/2 | 2/2 | 2/2 | 1/1 |

## Rows the lookup guard marked

A `remote` mark is a pattern in a command (github.com, gh, git clone) and is read by hand; it doesn't change the score. A `checkout` mark means the run read a source checkout and scored 0.

| task | config | round | marks |
|---|---|---|---|
| squawk-hard | opus high | 1 | remote |
| squawk-hard | sonnet xhigh | 1 | remote |
