-- Manifest/tag agreement: does the manifest declare the release, and does
-- the changelog name it? Pure logic over file contents, so `*_test.lua`
-- covers it directly; the release wrapper and workflow feed it file paths
-- through argv (see below).

local fs = airsstack.fs

local agreement = {}

--- The first `version = "..."` value in a manifest, or nil when absent.
---@param text string manifest contents
---@return string|nil
function agreement.manifest_version(text)
  for line in (text .. "\n"):gmatch("([^\n]*)\n") do
    local version = line:match('^%s*version%s*=%s*"([^"]+)"')
    if version ~= nil then
      return version
    end
  end
  return nil
end

--- Whether the manifest declares `version`.
---@param text string manifest contents
---@param version string the release version
---@return boolean ok, string detail
function agreement.check(text, version)
  local declared = agreement.manifest_version(text)
  if declared == nil then
    return false, "manifest has no version"
  end
  if declared ~= version then
    return false, "manifest declares " .. declared .. ", want " .. version
  end
  return true, "manifest declares " .. version
end

--- Whether the changelog has a `## <version>` entry heading, in plain or
--- bracketed Keep-a-Changelog form (`## [<version>] - <date>`).
---@param text string changelog contents
---@param version string the release version
---@return boolean
function agreement.has_entry(text, version)
  local plain = "## " .. version
  local bracketed = "## [" .. version .. "]"
  for line in (text .. "\n"):gmatch("([^\n]*)\n") do
    if line == plain or line:sub(1, #bracketed) == bracketed then
      return true
    end
  end
  return false
end

local function main(manifest_path, changelog_path, version)
  local ok, manifest = pcall(fs.read, manifest_path)
  if not ok then
    print("agreement: FAIL cannot read manifest: " .. tostring(manifest))
    return 1
  end
  local checked, detail = agreement.check(manifest, version)
  if not checked then
    print("agreement: FAIL " .. detail)
    return 1
  end
  local changelog_ok, changelog = pcall(fs.read, changelog_path)
  if not changelog_ok then
    print("agreement: FAIL cannot read changelog: " .. tostring(changelog))
    return 1
  end
  if not agreement.has_entry(changelog, version) then
    print("agreement: FAIL no '## " .. version .. "' entry in changelog")
    return 1
  end
  print("agreement: ok, manifest and changelog name " .. version)
  return 0
end

local function fail(code)
  -- os.exit is unavailable under some policies; a failing assert ends the
  -- run with a non-zero status instead.
  assert(code == 0, "release agreement failed")
end

if arg ~= nil and arg[0] ~= nil and arg[0]:sub(-#"agreement.lua") == "agreement.lua" then
  fail(main(arg[1], arg[2], arg[3]))
end

return agreement
