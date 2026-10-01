# easy: 32 runs

Cost is the CLI's list-price `total_cost_usd`. A run passes when it meets every check of its task; score is the share of checks met. Tasks marked private come from repos that aren't public, so their prompts, tests and diffs aren't in this repo; their numbers are.

## Per config

| config | runs | passed | checks met | mean score | said done but failed | timeouts | wall, mean | wall, median | cost, mean | tool calls, median | output tokens, median | lines +/−, median |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| sonnet medium | 8 | 7 | 51/52 | 0.979 | 1 | 0 | 46 s | 38 s | $0.18 | 7 | 4051 | +71 / −5 |
| sonnet high | 8 | 8 | 52/52 | 1.000 | 0 | 0 | 77 s | 70 s | $0.28 | 12 | 7384 | +169 / −8 |
| opus medium | 4 | 4 | 26/26 | 1.000 | 0 | 0 | 2.7 min | 2.9 min | $0.89 | 12 | 15784 | +229 / −17 |
| opus high | 4 | 4 | 26/26 | 1.000 | 0 | 0 | 3.6 min | 3.8 min | $1.08 | 16 | 19188 | +209 / −14 |
| fable low | 8 | 7 | 51/52 | 0.982 | 1 | 0 | 110 s | 83 s | $1.27 | 9 | 7133 | +185 / −10 |

## Score by task and round

| task | sonnet medium | sonnet high | opus medium | opus high | fable low |
|---|---|---|---|---|---|
| bridge-host-bug | 1 / 1 | 1 / 1 | 1 | 1 | 1 / 1 |
| flashies-practice-spec (private) | 1 / 1 | 1 / 1 | 1 | 1 | 1 / 1 |
| flashies-review-bug (private) | 1 / 0.83 | 1 / 1 | 1 | 1 | 1 / 1 |
| flashies-shortcuts-feature (private) | 1 / 1 | 1 / 1 | 1 | 1 | 0.86 / 1 |

## Wall time by task and round

| task | sonnet medium | sonnet high | opus medium | opus high | fable low |
|---|---|---|---|---|---|
| bridge-host-bug | 33 s / 37 s | 63 s / 45 s | 3.3 min | 4.6 min | 3.8 min / 76 s |
| flashies-practice-spec (private) | 28 s / 30 s | 53 s / 85 s | 90 s | 2.1 min | 81 s / 84 s |
| flashies-review-bug (private) | 50 s / 38 s | 74 s / 65 s | 2.5 min | 3.4 min | 74 s / 82 s |
| flashies-shortcuts-feature (private) | 77 s / 71 s | 2.1 min / 108 s | 3.3 min | 4.2 min | 2.2 min / 120 s |

## Checks, by how many runs met them

| task | check | sonnet medium | sonnet high | opus medium | opus high | fable low |
|---|---|---|---|---|---|---|
| bridge-host-bug | existing_tests_pass | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| bridge-host-bug | subagent_steps_start_under_parent | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| bridge-host-bug | subagent_steps_end | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| bridge-host-bug | subagent_chatter_stays_out | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| bridge-host-bug | agent_step_shows_reply | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| bridge-host-bug | reply_shown_in_history | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-practice-spec (private) | shuffled_same_cards | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-practice-spec (private) | shuffled_new_sequence | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-practice-spec (private) | shuffled_decks_interleaved | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-practice-spec (private) | stored_order_parsed | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-practice-spec (private) | strings_in_three_locales | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-practice-spec (private) | app_still_typechecks | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-practice-spec (private) | existing_tests_pass | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-review-bug (private) | existing_tests_still_pass | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-review-bug (private) | spacing_unchanged | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-review-bug (private) | first_got_it_turns_green | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-review-bug (private) | overruled_got_it_turns_green | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-review-bug (private) | server_rule_migrated | 1/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-review-bug (private) | stored_cards_backfilled | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-shortcuts-feature (private) | untouched_tests_pass | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-shortcuts-feature (private) | app_still_typechecks | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-shortcuts-feature (private) | second_key_binds_and_fires | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-shortcuts-feature (private) | one_owner_per_key | 2/2 | 2/2 | 1/1 | 1/1 | 1/2 |
| flashies-shortcuts-feature (private) | saved_remaps_stay_readable | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-shortcuts-feature (private) | hint_and_legend_follow | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
| flashies-shortcuts-feature (private) | dialog_offers_second_key | 2/2 | 2/2 | 1/1 | 1/1 | 2/2 |
