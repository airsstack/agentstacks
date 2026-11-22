use similar::TextDiff;

const OLD: &str = "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ]\n  }\n}\n";
const NEW: &str = "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ]\n  },\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"airsl run .claude/hooks/end.lua\"\n          }\n        ]\n      }\n    ]\n  }\n}\n";

fn main() {
    let diff = TextDiff::from_lines(OLD, NEW);
    let out = diff.unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== Debug (exact bytes):\n{out:?}");
    println!("=== text:");
    print!("{out}");
    println!("=== end");
    let same = TextDiff::from_lines(OLD, OLD).unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== identical inputs -> {same:?}");
    // file created from nothing (old empty)
    let created = TextDiff::from_lines("", "x\ny\n").unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== empty old -> {created:?}");
    // missing trailing newline on new
    let nonl = TextDiff::from_lines("a\n", "a\nb").unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== new without trailing newline -> {nonl:?}");
}
