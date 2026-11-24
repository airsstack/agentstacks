-- Tests for style (the hook module) and lib/style — the reply-rules section and the
-- style_reinject option.
--
--   cargo make lua

local hook = require("style")
local style = require("lib.style")
local fs = airsstack.fs

-- Path relative to the repository root, where `cargo make lua` runs.
local PROTOCOL = "packs/core/skills/discuss/references/protocol.md"

local DOC = table.concat({
  "# Communication Protocol",
  "",
  "Intro.",
  "",
  "## Reply rules",
  "",
  "1. one",
  "2. two",
  "",
  "## Brief",
  "",
  "not this",
}, "\n")

return {
  the_reply_rules_section_is_extracted_with_its_heading = function()
    assert(style.reply_rules(DOC) == "## Reply rules\n\n1. one\n2. two")
  end,

  extraction_stops_at_a_top_level_heading_too = function()
    local doc = "## Reply rules\n\nkeep\n# Other\ndrop"
    assert(style.reply_rules(doc) == "## Reply rules\n\nkeep")
  end,

  a_subheading_inside_the_section_is_kept = function()
    local doc = "## Reply rules\n\nkeep\n### detail\nalso keep\n## Next\ndrop"
    assert(style.reply_rules(doc) == "## Reply rules\n\nkeep\n### detail\nalso keep")
  end,

  no_section_means_nothing_to_print = function()
    assert(style.reply_rules("# Title\n\n## Brief\n\ntext") == nil)
  end,

  an_empty_section_means_nothing_to_print = function()
    assert(style.reply_rules("## Reply rules\n\n\n## Brief\nx") == nil)
  end,

  an_absent_option_means_reinject = function()
    assert(style.reinject_enabled(nil) == true)
  end,

  true_and_one_mean_reinject = function()
    assert(style.reinject_enabled("true") == true)
    assert(style.reinject_enabled("1") == true)
  end,

  false_and_zero_turn_it_off_whatever_the_case_or_spacing = function()
    assert(style.reinject_enabled("false") == false)
    assert(style.reinject_enabled("0") == false)
    assert(style.reinject_enabled(" FALSE ") == false)
  end,

  an_unreadable_value_keeps_the_default = function()
    -- A placeholder or junk is not an instruction to stop; on is the default.
    assert(style.reinject_enabled("{{option style_reinject}}") == true)
    assert(style.reinject_enabled("") == true)
  end,

  the_shipped_protocol_carries_the_four_rules_verbatim = function()
    local text = fs.read(PROTOCOL)
    local rules = assert(style.reply_rules(text), "no ## Reply rules section")
    for _, rule in ipairs({
      "1. Reduce agent verbosity output.",
      "2. Do not over-explain everything; only explain what matters.",
      "3. Use ASCII visualizations to replace long narratives or text.",
      "4. Be concise but precise; only tell what matters and is important.",
      "- The outcome comes first.",
      "- Anything that would run past a short paragraph becomes an ASCII diagram, table or tree.",
      "- While a discussion is open, a reply ends with its topic list.",
    }) do
      assert(rules:find(rule, 1, true), "missing: " .. rule)
    end
    assert(not rules:find("comm-protocol:", 1, true), "the section ran into ## Brief")
  end,

  session_start_always_returns_the_rules = function()
    local rules = assert(style.reply_rules(fs.read(PROTOCOL)))
    local got = hook({ event = "session.start", args = { PROTOCOL } })
    assert(type(got) == "table" and got.context == rules, "no rules on session.start")
  end,

  session_start_ignores_a_false_option = function()
    local rules = assert(style.reply_rules(fs.read(PROTOCOL)))
    local got = hook({ event = "session.start", args = { PROTOCOL, "false" } })
    assert(type(got) == "table" and got.context == rules, "session.start must always send")
  end,

  prompt_submit_with_false_returns_nil = function()
    assert(hook({ event = "prompt.submit", args = { PROTOCOL, "false" } }) == nil)
    assert(hook({ event = "prompt.submit", args = { PROTOCOL, "0" } }) == nil)
  end,

  prompt_submit_with_true_returns_the_rules = function()
    local rules = assert(style.reply_rules(fs.read(PROTOCOL)))
    local got = hook({ event = "prompt.submit", args = { PROTOCOL, "true" } })
    assert(type(got) == "table" and got.context == rules, "no rules on prompt.submit")
  end,

  prompt_submit_without_an_option_returns_the_rules = function()
    local rules = assert(style.reply_rules(fs.read(PROTOCOL)))
    local got = hook({ event = "prompt.submit", args = { PROTOCOL } })
    assert(type(got) == "table" and got.context == rules)
  end,

  no_protocol_path_returns_nil = function()
    assert(hook({ event = "session.start", args = {} }) == nil)
    assert(hook({ event = "session.start", args = { "" } }) == nil)
  end,

  an_unreadable_protocol_returns_nil = function()
    assert(hook({ event = "session.start", args = { "packs/core/no/such/protocol.md" } }) == nil)
  end,
}
