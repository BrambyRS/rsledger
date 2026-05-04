use serde::Deserialize;
use std::path::PathBuf;

/// RULE ACTION
/// Possible actions for the RegexRule
/// AssignAccount: assign the transaction to the specified account
/// Skip: skip the transaction (do not import it)
pub enum RuleAction {
    AssignAccount(String),
    Skip,
}

/// REGEX RULE
/// Regexc rule with a pattern and an associated action to take on match.
pub struct RegexRule {
    pub pattern: regex::Regex,
    pub action: RuleAction,
}

/// REGEX RULE FROM FILE
/// Intermediate struct for deserialising RegexRule from file.
/// This is needed because regex::Regex does not implement Deserialize
/// so we need to parse the pattern as a string
#[derive(Deserialize)]
struct RegexRuleFromFile {
    pattern: String,
    action: String,
    account: Option<String>,
}

/// RULE SHEET FILE
/// Struct for deserialising the whole rule sheet from file.
#[derive(Deserialize)]
struct RuleSheetFile {
    rules: Vec<RegexRuleFromFile>,
}

/// READ_RULE_SHEET
/// Reads a rule sheet `.toml` file from the specified path and returns a vector of `RegexRule`s.
///
/// The rule sheet should be a TOML file with the following format:
/// ```toml
/// [[rules]]
/// pattern = "regex pattern to match against transaction descriptions"
/// action = "assign_account" # or "skip"
/// account = "account:name" # required if action is "assign_account", ignored if action is "skip"
/// ```
pub fn read_rule_sheet(path: PathBuf) -> crate::Result<Vec<RegexRule>> {
    let rule_sheet_str = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading rule sheet {}: {}", path.display(), e);
            return Err(crate::error::RsledgerError::ParseError(
                "Rule Sheet".to_string(),
                format!("Error reading rule sheet {}: {}", path.display(), e),
            ));
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
                return Err(crate::error::RsledgerError::ParseError(
                    "Rule Sheet".to_string(),
                    format!("Error parsing rule sheet {}: {}", path.display(), e),
                ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_rule_sheet(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("rule_sheets")
            .join(name)
    }

    #[test]
    fn valid_rules_are_parsed() {
        let rules = read_rule_sheet(test_rule_sheet("valid_rules.toml")).unwrap();
        assert_eq!(rules.len(), 3);

        assert!(rules[0].pattern.is_match("GROCERY STORE #123"));
        assert!(
            matches!(&rules[0].action, RuleAction::AssignAccount(a) if a == "expenses:food:groceries")
        );

        assert!(rules[1].pattern.is_match("NETFLIX subscription"));
        assert!(rules[1].pattern.is_match("SPOTIFY premium"));
        assert!(
            matches!(&rules[1].action, RuleAction::AssignAccount(a) if a == "expenses:entertainment:subscriptions")
        );

        assert!(rules[2].pattern.is_match("INTERNAL TRANSFER to savings"));
        assert!(matches!(&rules[2].action, RuleAction::Skip));
    }

    #[test]
    fn missing_account_skips_rule() {
        let rules = read_rule_sheet(test_rule_sheet("missing_account.toml")).unwrap();
        // First rule (missing account for assign_account) should be skipped
        assert_eq!(rules.len(), 1);
        assert!(rules[0].pattern.is_match("RENT PAYMENT"));
        assert!(
            matches!(&rules[0].action, RuleAction::AssignAccount(a) if a == "expenses:housing:rent")
        );
    }

    #[test]
    fn invalid_regex_skips_rule() {
        let rules = read_rule_sheet(test_rule_sheet("invalid_regex.toml")).unwrap();
        // First rule (invalid regex) should be skipped, second should remain
        assert_eq!(rules.len(), 1);
        assert!(rules[0].pattern.is_match("VALID PATTERN xyz"));
        assert!(matches!(&rules[0].action, RuleAction::Skip));
    }

    #[test]
    fn empty_rules_returns_empty_vec() {
        let rules = read_rule_sheet(test_rule_sheet("empty_rules.toml")).unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn malformed_toml_returns_error() {
        let result = read_rule_sheet(test_rule_sheet("malformed.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let result = read_rule_sheet(test_rule_sheet("does_not_exist.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn skip_action_ignores_account_field() {
        let rules = read_rule_sheet(test_rule_sheet("skip_with_account.toml")).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(matches!(&rules[0].action, RuleAction::Skip));
    }

    #[test]
    fn action_matching_is_case_insensitive() {
        let rules = read_rule_sheet(test_rule_sheet("case_insensitive_action.toml")).unwrap();
        assert_eq!(rules.len(), 2);
        assert!(matches!(&rules[0].action, RuleAction::Skip));
        assert!(matches!(&rules[1].action, RuleAction::Skip));
    }
}
