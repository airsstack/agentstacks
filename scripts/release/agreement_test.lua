-- The release gate's agreement checks: manifest version and changelog entry.

local agreement = require("agreement")

local function reads_the_first_version()
  local text = '[pack]\nname = "core"\nversion = "1.0.1"\napi = 1\n'
  assert(agreement.manifest_version(text) == "1.0.1")
  assert(agreement.manifest_version("[pack]\nname = \"core\"\n") == nil)
end

local function check_matches_exactly()
  local text = '[pack]\nname = "core"\nversion = "1.0.1"\napi = 1\n'
  local ok, detail = agreement.check(text, "1.0.1")
  assert(ok, detail)
  local bad, why = agreement.check(text, "1.0.2")
  assert(not bad)
  assert(why == "manifest declares 1.0.1, want 1.0.2", why)
  local missing, reason = agreement.check("[pack]\nname = \"core\"\n", "1.0.1")
  assert(not missing)
  assert(reason == "manifest has no version", reason)
end

local function entry_needs_the_exact_heading()
  local text = "# Changelog\n\n## 1.0.1\n\n- Fixed.\n"
  assert(agreement.has_entry(text, "1.0.1"))
  assert(not agreement.has_entry(text, "1.0.2"))
  assert(not agreement.has_entry("# Changelog\n\nNothing here.\n", "1.0.1"))
end

local function entry_accepts_the_bracketed_form()
  local text = "# Changelog\n\n## [1.0.1] - 2026-11-27\n\n- Fixed.\n"
  assert(agreement.has_entry(text, "1.0.1"))
  assert(not agreement.has_entry(text, "1.0.2"))
  assert(not agreement.has_entry("# Changelog\n\n## [1.0.10] - 2026-11-27\n", "1.0.1"))
end

return {
  reads_the_first_version = reads_the_first_version,
  check_matches_exactly = check_matches_exactly,
  entry_needs_the_exact_heading = entry_needs_the_exact_heading,
  entry_accepts_the_bracketed_form = entry_accepts_the_bracketed_form,
}
