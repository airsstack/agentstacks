-- The release gate's type checks: every subject is a feature, a fix, or
-- neither, and features outrank fixes.

local suggest = require("suggest")

local function parses()
  local cases = {
    ["feat(packs): add streams"] = "feat",
    ["feat!: break the tag"] = "feat",
    ["feat: no scope"] = "feat",
    ["fix(agentstacks): stop the leak"] = "fix",
    ["fix: one-liner"] = "fix",
    ["chore(repo): release 1.2.0"] = nil,
    ["docs: rewrite the guide"] = nil,
    ["Merge pull request #24 from airsstack/x"] = nil,
    ["feature creep"] = nil,
    ["fixfoo: almost"] = nil,
    ["Feat: capitalized"] = nil,
  }
  for subject, want in pairs(cases) do
    local got = suggest.parse_subject(subject)
    assert(
      got == want,
      "parse " .. subject .. ": want " .. tostring(want) .. ", got " .. tostring(got)
    )
  end
end

local function suggests_minor_for_any_feature()
  assert(suggest.from_subjects({ "fix: a", "feat: b", "chore: c" }) == "minor")
  assert(suggest.from_subjects({ "feat: only" }) == "minor")
end

local function suggests_patch_for_fixes_only()
  assert(suggest.from_subjects({ "fix: a", "docs: b" }) == "patch")
end

local function suggests_nothing_without_feat_or_fix()
  assert(suggest.from_subjects({}) == nil)
  assert(suggest.from_subjects({ "chore: a", "docs: b" }) == nil)
end

return {
  parses_conventional_subjects = parses,
  suggests_minor_for_any_feature = suggests_minor_for_any_feature,
  suggests_patch_for_fixes_only = suggests_patch_for_fixes_only,
  suggests_nothing_without_feat_or_fix = suggests_nothing_without_feat_or_fix,
}
