//! Reading the table an extension's `values` handler returned.
//!
//! # Responsibilities
//!
//! - [`ValuesResult::read`] checks the table key by key and collects every refusal: `root` is
//!   required and is a valid path; `skills`, `agents` and `vars` may be absent and are tables
//!   when present; each key names a skill, an agent or a var of this run; a skill entry holds a
//!   string `invoke` and a valid path `dir` and nothing else; an agent value and a var value are
//!   strings; no other key is allowed.
//! - The accepted answer holds every path normalised and is read through [`ValuesResult::root`],
//!   [`ValuesResult::skill`], [`ValuesResult::agent`] and [`ValuesResult::var`].
//!
//! # Non-responsibilities
//!
//! The word a refusal gives for the type of a value comes from the caller, so the word rule
//! exists once. A handler that is missing, or that returned something other than a table, is
//! refused by the extension host before this is called.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde_json::{Map, Value};

use crate::types::name::{ItemName, PackName};
use crate::types::op::payload::ValuesInput;
use crate::types::op::{LuaType, OpError, OpPath};
use crate::types::report::{Diagnostic, Diagnostics, FieldError, FormatError, Location};
use crate::types::tokens::Var;

/// The keys the table may hold at the top.
const TOP_KEYS: [&str; 4] = ["root", "skills", "agents", "vars"];

/// The keys a skill entry may hold.
const SKILL_KEYS: [&str; 2] = ["invoke", "dir"];

/// Where a skill is installed and how it is invoked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillValues {
    /// How the tool invokes the skill (`/status`).
    pub invoke: String,
    /// The directory the tool installs the skill in.
    pub dir: OpPath,
}

/// A checked `values` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuesResult {
    root: OpPath,
    skills: BTreeMap<(PackName, ItemName), SkillValues>,
    agents: BTreeMap<(PackName, ItemName), String>,
    vars: HashMap<Var, String>,
}

/// Which kind of item a `values` key names.
#[derive(Debug, Clone, Copy)]
enum ItemKind {
    Skill,
    Agent,
}

/// The state of one read: where refusals are located and what they are collected into.
struct Reader<'a, F> {
    entry: &'a Path,
    input: &'a ValuesInput,
    word: F,
    found: Diagnostics,
}

impl<F: Fn(&Value) -> LuaType> Reader<'_, F> {
    /// Refuses at the key path `key` of the extension entry.
    fn refuse_at(&mut self, key: &str, error: impl Into<FormatError>) {
        let location = Location::at_key(self.entry, key);
        self.found.push(Diagnostic::new(location, error));
    }

    /// The string at `key`; a value of another type is refused.
    fn string(&mut self, key: &str, value: &Value) -> Option<String> {
        if let Value::String(text) = value {
            return Some(text.clone());
        }
        let got = (self.word)(value);
        let expected = LuaType::String;
        self.refuse_at(key, OpError::ValuesType { expected, got });
        None
    }

    /// The string at `key`, which must be present and not `null`.
    fn required_string(&mut self, key: &str, value: Option<&Value>) -> Option<String> {
        match value {
            None | Some(Value::Null) => {
                self.refuse_at(key, FieldError::Required);
                None
            }
            Some(other) => self.string(key, other),
        }
    }

    /// The path at `key`, which must be present, a string and a valid project path.
    fn required_path(&mut self, key: &str, value: Option<&Value>) -> Option<OpPath> {
        let text = self.required_string(key, value)?;
        match OpPath::new(&text) {
            Ok(path) => Some(path),
            Err(shape) => {
                let error = OpError::ValuesPath {
                    key: key.to_owned(),
                    shape,
                };
                self.found
                    .push(Diagnostic::new(Location::file(self.entry), error));
                None
            }
        }
    }

    /// The table at `key`; `None` when it is absent or is another type, which is refused.
    fn table<'v>(&mut self, key: &str, value: Option<&'v Value>) -> Option<&'v Map<String, Value>> {
        match value? {
            Value::Object(table) => Some(table),
            other => {
                let got = (self.word)(other);
                let expected = LuaType::Table;
                self.refuse_at(key, OpError::ValuesType { expected, got });
                None
            }
        }
    }

    /// The `<pack>/<item>` a key names, when the run has it; otherwise the key is refused at
    /// `path`.
    fn item_key(&mut self, key: &str, path: &str, kind: ItemKind) -> Option<(PackName, ItemName)> {
        let named = key
            .split_once('/')
            .and_then(|(pack, item)| Some((PackName::new(pack).ok()?, ItemName::new(item).ok()?)));
        let listed = named.filter(|(pack, item)| match kind {
            ItemKind::Skill => self.input.has_skill(pack, item),
            ItemKind::Agent => self.input.has_agent(pack, item),
        });
        if listed.is_none() {
            let error = match kind {
                ItemKind::Skill => OpError::NotASkill,
                ItemKind::Agent => OpError::NotAnAgent,
            };
            self.refuse_at(path, error);
        }
        listed
    }

    fn skills(&mut self, value: Option<&Value>) -> BTreeMap<(PackName, ItemName), SkillValues> {
        let mut skills = BTreeMap::new();
        let Some(table) = self.table("values.skills", value) else {
            return skills;
        };
        for (key, entry) in table {
            let path = format!("values.skills.\"{key}\"");
            let Some(id) = self.item_key(key, &path, ItemKind::Skill) else {
                continue;
            };
            if let Some(values) = self.skill_entry(&path, entry) {
                skills.insert(id, values);
            }
        }
        skills
    }

    fn skill_entry(&mut self, path: &str, entry: &Value) -> Option<SkillValues> {
        let table = self.table(path, Some(entry))?;
        let invoke = self.required_string(&format!("{path}.invoke"), table.get("invoke"));
        let dir = self.required_path(&format!("{path}.dir"), table.get("dir"));
        for key in table
            .keys()
            .filter(|key| !SKILL_KEYS.contains(&key.as_str()))
        {
            self.refuse_at(&format!("{path}.{key}"), FieldError::UnknownKey);
        }
        Some(SkillValues {
            invoke: invoke?,
            dir: dir?,
        })
    }

    fn agents(&mut self, value: Option<&Value>) -> BTreeMap<(PackName, ItemName), String> {
        let mut agents = BTreeMap::new();
        let Some(table) = self.table("values.agents", value) else {
            return agents;
        };
        for (key, entry) in table {
            let path = format!("values.agents.\"{key}\"");
            let Some(id) = self.item_key(key, &path, ItemKind::Agent) else {
                continue;
            };
            if let Some(text) = self.string(&path, entry) {
                agents.insert(id, text);
            }
        }
        agents
    }

    fn vars(&mut self, value: Option<&Value>) -> HashMap<Var, String> {
        let mut vars = HashMap::new();
        let Some(table) = self.table("values.vars", value) else {
            return vars;
        };
        for (key, entry) in table {
            let path = format!("values.vars.{key}");
            let Some(var) = Var::new(key).ok().filter(|var| self.input.has_var(*var)) else {
                self.refuse_at(&path, OpError::NotAVar);
                continue;
            };
            if let Some(text) = self.string(&path, entry) {
                vars.insert(var, text);
            }
        }
        vars
    }
}

impl ValuesResult {
    /// Reads the `values` table `object` the extension returned, for the run `input`.
    ///
    /// `entry` is the extension's entry file as refusals show it (`<tool>/<entry file>`), and
    /// `word` names the Lua type of a value, for a refusal that says what was given.
    ///
    /// # Errors
    ///
    /// Every refusal found, located at the entry file with the key path from `values`.
    pub fn read<F>(
        object: &Map<String, Value>,
        input: &ValuesInput,
        entry: &Path,
        word: F,
    ) -> Result<Self, Diagnostics>
    where
        F: Fn(&Value) -> LuaType,
    {
        let mut reader = Reader {
            entry,
            input,
            word,
            found: Diagnostics::new(),
        };
        let root = reader.required_path("values.root", object.get("root"));
        let skills = reader.skills(object.get("skills"));
        let agents = reader.agents(object.get("agents"));
        let vars = reader.vars(object.get("vars"));
        for key in object
            .keys()
            .filter(|key| !TOP_KEYS.contains(&key.as_str()))
        {
            reader.refuse_at(&format!("values.{key}"), FieldError::UnknownKey);
        }
        match root {
            Some(root) => reader.found.into_result(Self {
                root,
                skills,
                agents,
                vars,
            }),
            None => Err(reader.found),
        }
    }

    /// The directory the extension's root files and the hook runtime are written under.
    #[must_use]
    pub const fn root(&self) -> &OpPath {
        &self.root
    }

    /// Where `skill` of `pack` is installed and how it is invoked, when the extension said.
    #[must_use]
    pub fn skill(&self, pack: &PackName, skill: &ItemName) -> Option<&SkillValues> {
        self.skills.get(&(pack.clone(), skill.clone()))
    }

    /// How `agent` of `pack` is written, when the extension said.
    #[must_use]
    pub fn agent(&self, pack: &PackName, agent: &ItemName) -> Option<&str> {
        self.agents
            .get(&(pack.clone(), agent.clone()))
            .map(String::as_str)
    }

    /// The value of `var`, when the extension gave one.
    #[must_use]
    pub fn var(&self, var: Var) -> Option<&str> {
        self.vars.get(&var).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::types::op::testing::{json_word, values_input};

    const ENTRY: &str = "claude/adapter.lua";

    fn run(json: &str) -> Result<Result<ValuesResult, Vec<String>>, Box<dyn Error>> {
        let value: Value = serde_json::from_str(json)?;
        let object = value.as_object().ok_or("the result is an object")?;
        let read = ValuesResult::read(object, &values_input()?, Path::new(ENTRY), json_word);
        Ok(read.map_err(|found| {
            found
                .into_sorted()
                .iter()
                .map(ToString::to_string)
                .collect()
        }))
    }

    fn accepted(json: &str) -> Result<ValuesResult, Box<dyn Error>> {
        run(json)?.map_err(|found| found.join("; ").into())
    }

    fn refused(json: &str) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(run(json)?.err().ok_or("a refusal")?)
    }

    fn one(json: &str) -> Result<String, Box<dyn Error>> {
        let mut found = refused(json)?;
        if found.len() == 1 {
            Ok(found.remove(0))
        } else {
            Err(format!("expected one refusal, got {found:?}").into())
        }
    }

    #[test]
    fn values_example_result_is_accepted() -> Result<(), Box<dyn Error>> {
        let result = accepted(
            r#"{"agents":{"core/coder":"coder"},"root":".claude/agentstacks","skills":{"core/status":{"dir":".claude/skills/status","invoke":"/status"}},"vars":{"tmpdir":"$TMPDIR"}}"#,
        )?;
        let core = PackName::new("core")?;
        assert_eq!(result.root().as_str(), ".claude/agentstacks");
        let skill = result
            .skill(&core, &ItemName::new("status")?)
            .ok_or("the skill entry")?;
        assert_eq!(skill.invoke, "/status");
        assert_eq!(skill.dir.as_str(), ".claude/skills/status");
        assert_eq!(result.agent(&core, &ItemName::new("coder")?), Some("coder"));
        assert_eq!(result.var(Var::Tmpdir), Some("$TMPDIR"));
        assert_eq!(result.var(Var::SessionId), None);
        Ok(())
    }

    #[test]
    fn values_root_is_required() -> Result<(), Box<dyn Error>> {
        assert_eq!(one("{}")?, "claude/adapter.lua: values.root: required");
        assert_eq!(
            one(r#"{"root":null}"#)?,
            "claude/adapter.lua: values.root: required"
        );
        Ok(())
    }

    #[test]
    fn values_root_must_be_a_valid_path() -> Result<(), Box<dyn Error>> {
        let cases = [
            (r#"{"root":"/x"}"#, "\"/x\" must be relative"),
            (r#"{"root":"."}"#, "empty"),
            (r#"{"root":""}"#, "empty"),
            (
                r#"{"root":"../x"}"#,
                "\"../x\" must stay inside the project",
            ),
            (r#"{"root":"a\\b"}"#, "\"a\\b\" must use / as the separator"),
        ];
        for (json, shape) in cases {
            assert_eq!(
                one(json)?,
                format!("claude/adapter.lua: values.root: {shape}")
            );
        }
        Ok(())
    }

    #[test]
    fn values_root_may_be_an_own_file_name() -> Result<(), Box<dyn Error>> {
        let result = accepted(r#"{"root":"agentstacks.lock"}"#)?;
        assert_eq!(result.root().as_str(), "agentstacks.lock");
        Ok(())
    }

    #[test]
    fn values_root_must_be_a_string() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":5}"#)?,
            "claude/adapter.lua: values.root: expected string, got number"
        );
        Ok(())
    }

    #[test]
    fn values_paths_are_normalised() -> Result<(), Box<dyn Error>> {
        let result = accepted(
            r#"{"root":"./.claude//agentstacks","skills":{"core/status":{"invoke":"/s","dir":".claude/skills/./status"}}}"#,
        )?;
        assert_eq!(result.root().as_str(), ".claude/agentstacks");
        let skill = result
            .skill(&PackName::new("core")?, &ItemName::new("status")?)
            .ok_or("the skill entry")?;
        assert_eq!(skill.dir.as_str(), ".claude/skills/status");
        Ok(())
    }

    #[test]
    fn values_tables_refuse_a_list() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","agents":["x"]}"#)?,
            "claude/adapter.lua: values.agents: expected table, got list"
        );
        Ok(())
    }

    #[test]
    fn values_tables_may_be_absent() -> Result<(), Box<dyn Error>> {
        let result = accepted(r#"{"root":"r"}"#)?;
        let core = PackName::new("core")?;
        assert_eq!(result.skill(&core, &ItemName::new("status")?), None);
        assert_eq!(result.agent(&core, &ItemName::new("coder")?), None);
        assert_eq!(result.var(Var::Tmpdir), None);
        Ok(())
    }

    #[test]
    fn values_refuses_a_skill_not_in_this_run() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","skills":{"core/x":{"invoke":"/x","dir":"d"}}}"#)?,
            "claude/adapter.lua: values.skills.\"core/x\": not a skill in this run"
        );
        Ok(())
    }

    #[test]
    fn values_refuses_an_agent_not_in_this_run() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","agents":{"core/x":"x"}}"#)?,
            "claude/adapter.lua: values.agents.\"core/x\": not an agent in this run"
        );
        Ok(())
    }

    #[test]
    fn values_refuses_a_var_not_in_this_run() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","vars":{"home":"h"}}"#)?,
            "claude/adapter.lua: values.vars.home: not a var in this run"
        );
        Ok(())
    }

    #[test]
    fn values_skill_entry_needs_invoke_and_dir() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","skills":{"core/status":{"invoke":"/status"}}}"#)?,
            "claude/adapter.lua: values.skills.\"core/status\".dir: required"
        );
        assert_eq!(
            one(r#"{"root":"r","skills":{"core/status":{"dir":"d"}}}"#)?,
            "claude/adapter.lua: values.skills.\"core/status\".invoke: required"
        );
        Ok(())
    }

    #[test]
    fn values_skill_entry_must_be_a_table() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","skills":{"core/status":"/status"}}"#)?,
            "claude/adapter.lua: values.skills.\"core/status\": expected table, got string"
        );
        Ok(())
    }

    #[test]
    fn values_skill_fields_must_be_strings() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","skills":{"core/status":{"invoke":"/s","dir":5}}}"#)?,
            "claude/adapter.lua: values.skills.\"core/status\".dir: expected string, got number"
        );
        Ok(())
    }

    #[test]
    fn values_skill_dir_must_be_a_valid_path() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","skills":{"core/status":{"invoke":"/s","dir":"../x"}}}"#)?,
            "claude/adapter.lua: values.skills.\"core/status\".dir: \"../x\" must stay inside the project"
        );
        Ok(())
    }

    #[test]
    fn values_agent_and_var_values_must_be_strings() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            refused(r#"{"root":"r","agents":{"core/coder":true},"vars":{"tmpdir":{}}}"#)?,
            [
                "claude/adapter.lua: values.agents.\"core/coder\": expected string, got boolean",
                "claude/adapter.lua: values.vars.tmpdir: expected string, got table",
            ]
        );
        Ok(())
    }

    #[test]
    fn values_refuses_unknown_keys_at_any_level() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            one(r#"{"root":"r","foo":1}"#)?,
            "claude/adapter.lua: values.foo: unknown key"
        );
        assert_eq!(
            one(r#"{"root":"r","skills":{"core/status":{"invoke":"/s","dir":"d","icon":"i"}}}"#)?,
            "claude/adapter.lua: values.skills.\"core/status\".icon: unknown key"
        );
        Ok(())
    }

    #[test]
    fn values_collects_every_refusal() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            refused(r#"{"root":"/x","foo":1,"vars":{"home":"h"}}"#)?,
            [
                "claude/adapter.lua: values.root: \"/x\" must be relative",
                "claude/adapter.lua: values.vars.home: not a var in this run",
                "claude/adapter.lua: values.foo: unknown key",
            ]
        );
        Ok(())
    }
}
