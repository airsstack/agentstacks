# Plugin Lua suites under airsl 0.1.4

Run 2026-11-23 from /Users/hiraq/Projects/airsstack/claudestacks. Grants are the ones in claudestacks Makefile.toml tasks.plugins-test. Last 6 lines of each run pasted verbatim.

~~~~text
$ airsl --version
airsl 0.1.4
~~~~

## claudestacks

~~~~text
$ airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git plugins/claudestacks
  ok    no_section_means_nothing_to_print
  ok    the_reply_rules_section_is_extracted_with_its_heading
  ok    the_shipped_protocol_carries_the_four_rules_verbatim
  ok    true_and_one_mean_reinject

265 passed, 0 failed (10 files)
exit=0
~~~~

## claudestacks-sdlc

~~~~text
$ airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git plugins/claudestacks-sdlc
  ok    the_triage_tag_follows_the_adr_parts
  ok    the_triage_tag_follows_the_sync_parts
  ok    unassigned_is_omitted_when_every_open_chain_is_a_member
  ok    unterminated_frontmatter_reports_a_reason

262 passed, 0 failed (4 files)
exit=0
~~~~

## claudestacks-journal

~~~~text
$ airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git plugins/claudestacks-journal
  ok    sort_rows_orders_element_by_element_like_a_tuple
  ok    sorted_keys_orders_the_keys_and_survives_an_empty_map
  ok    the_common_dir_of_a_plain_checkout_is_its_own_dot_git
  ok    unquote_strips_one_surrounding_pair_and_the_space_around_it

79 passed, 0 failed (6 files)
exit=0
~~~~

## claudestacks-guideline-rust

~~~~text
$ airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git plugins/claudestacks-guideline-rust
airsl: no test files found under `plugins/claudestacks-guideline-rust`
       a test file is named `*_test.lua` or `test_*.lua`
exit=1
~~~~

## claudestacks-guideline-python

~~~~text
$ airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git plugins/claudestacks-guideline-python
airsl: no test files found under `plugins/claudestacks-guideline-python`
       a test file is named `*_test.lua` or `test_*.lua`
exit=1
~~~~
