//! Pure static registry validation; no app rename or claim of live title discovery.
use serde::Serialize;
use std::{collections::BTreeSet,fs,path::Path};

#[derive(Serialize)]
pub struct Report { pub schema:&'static str,pub chats:usize,pub errors:Vec<String>,pub live_app_verified:bool }

pub fn validate(text:&str)->Report {
    let id=regex::Regex::new(r"^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$").expect("literal id pattern");
    let mut seen=BTreeSet::new();let mut titles=BTreeSet::new();let mut result=Report {schema:"humanymous.chat-registry.v1",chats:0,errors:Vec::new(),live_app_verified:false};
    for line in text.lines().filter(|l|l.starts_with('|')) {
        let cells=line.split('|').skip(1).map(str::trim).collect::<Vec<_>>();
        if cells.len()<3 || cells[0]=="Chat" || cells[0].starts_with('-') {continue;}
        result.chats+=1;let name=cells[0];let parts=name.split(" · ").collect::<Vec<_>>();
        if parts.len()!=3 || parts[0]!="Humanymous"||parts.iter().any(|p|p.trim().is_empty())||name.contains(['/', '\\']) {
            result.errors.push(format!("invalid project title: {name}"));
        }
        if !id.is_match(cells[1])||!seen.insert(cells[1].to_owned()) {result.errors.push(format!("invalid/duplicate chat id: {}",cells[1]));}
        if !titles.insert(name.to_owned()) {result.errors.push(format!("duplicate project title: {name}"));}
        if cells[2].is_empty() {result.errors.push(format!("missing responsibility: {name}"));}
    }
    if result.chats==0 {result.errors.push("empty registry".into());}result
}
pub fn inspect(root:&Path)->Result<Report,std::io::Error> {Ok(validate(&fs::read_to_string(root.join(".agents/sessions/THREADS.md"))?))}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn repository_registry_has_valid_stable_names_and_unique_ids() {
        let r=validate(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../.agents/sessions/THREADS.md")));assert!(r.errors.is_empty(),"{:?}",r.errors);assert!(!r.live_app_verified);
    }
    #[test] fn duplicate_ids_or_path_titles_are_not_accepted() {
        let text="| Chat | ID | Responsibility |\n|---|---|---|\n| Humanymous · tools · src/file | 01a1b34e-f9e9-77d3-b883-0cc4d76763a4 | x |\n| Humanymous · tools · skills | 01a1b34e-f9e9-77d3-b883-0cc4d76763a4 | x |";
        assert_eq!(validate(text).errors.len(),2);
    }
}
