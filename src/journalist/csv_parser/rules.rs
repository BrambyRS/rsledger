use serde::Deserialize;
use std::path::PathBuf;

pub enum RuleAction {
    AssignAccount(String),
    Skip,
}

pub struct RegexRule {
    pub pattern: regex::Regex,
    pub action: RuleAction,
}

#[derive(Deserialize)]
struct RegexRuleFromFile {
    pattern: String,
    action: String,
    account: Option<String>,
}

#[derive(Deserialize)]
struct RuleSheetFile {
    rules: Vec<RegexRuleFromFile>,
}

pub fn read_rule_sheet(path: PathBuf) -> Result<Vec<RegexRule>, Box<dyn std::error::Error>> {
    let rule_sheet_str = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading rule sheet {}: {}", path.display(), e);
            return Err(e.into());
        }
    };

    let rules_from_file: Vec<RegexRuleFromFile> =
        match toml::from_str::<RuleSheetFile>(&rule_sheet_str) {
            Ok(sheet) => sheet.rules,
            Err(e) => {
                eprintln!(
                    "Error parsing rule sheet {}: {}. Make sure it is a valid TOML file.",
                    path.display(),
                    e
                );
                return Err(e.into());
            }
        };

    let mut rules: Vec<RegexRule> = Vec::with_capacity(rules_from_file.len());
    for rule_from_file in rules_from_file {
        let action = if rule_from_file.action.to_lowercase() == "skip" {
            RuleAction::Skip
        } else {
            match rule_from_file.account {
                Some(account) => RuleAction::AssignAccount(account),
                None => {
                    eprintln!(
                        "Rule with pattern '{}' has action 'assign_account' but no account. Skipping.",
                        rule_from_file.pattern
                    );
                    continue;
                }
            }
        };
        match regex::Regex::new(&rule_from_file.pattern) {
            Ok(pattern) => rules.push(RegexRule { pattern, action }),
            Err(e) => eprintln!(
                "Error compiling regex pattern '{}': {}. Skipping this rule.",
                rule_from_file.pattern, e
            ),
        }
    }

    return Ok(rules);
}
