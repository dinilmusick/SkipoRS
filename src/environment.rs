//! Portable, fail-closed environment contract and migration planning for Skipo.
//! Pinned to Skipo 25a9921b051e7c6b2601fe33f0f163e432a56d2a,
//! architecture/environment/contract.json schema 2 (contract 2.0.0).
//! Only literal .env declarations are supported in this first Rust slice.

use std::collections::BTreeMap;

pub const REQUIRED: [&str; 2] = ["SYSTEM_DATABASE_URL", "APP_URL"];
pub const OPTIONAL: [&str; 2] = ["SYSTEM_DATABASE_URL_2", "SYSTEM_DATABASE_URL_3"];
pub const ALIASES: [(&str, &str); 2] = [
    ("DEPM_GATEWAY_DOMAIN", "APP_URL"),
    ("SYSTEM_DATABASE_URL_1", "SYSTEM_DATABASE_URL"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableClass {
    CanonicalRequired,
    CanonicalOptional,
    DeprecatedAlias(&'static str),
    RuntimePlatform,
    InternalTooling,
    Undeclared,
}

pub fn classify(name: &str) -> VariableClass {
    match name {
        "SYSTEM_DATABASE_URL" | "APP_URL" => VariableClass::CanonicalRequired,
        "SYSTEM_DATABASE_URL_2" | "SYSTEM_DATABASE_URL_3" => VariableClass::CanonicalOptional,
        "DEPM_GATEWAY_DOMAIN" => VariableClass::DeprecatedAlias("APP_URL"),
        "SYSTEM_DATABASE_URL_1" => VariableClass::DeprecatedAlias("SYSTEM_DATABASE_URL"),
        "NODE_ENV"
        | "NEXT_RUNTIME"
        | "PORT"
        | "CI"
        | "DISABLE_HMR"
        | "PATH"
        | "HOME"
        | "PWD"
        | "GITHUB_ACTIONS"
        | "GITHUB_ENV"
        | "GITHUB_OUTPUT"
        | "GITHUB_PATH"
        | "GITHUB_STEP_SUMMARY"
        | "GITHUB_WORKSPACE"
        | "GITHUB_REPOSITORY"
        | "GITHUB_REF"
        | "GITHUB_SHA"
        | "GITHUB_RUN_ID"
        | "VERCEL_ENV"
        | "VERCEL_URL"
        | "VERCEL_REGION"
        | "CF_PAGES"
        | "CF_PAGES_URL"
        | "CF_PAGES_BRANCH" => VariableClass::RuntimePlatform,
        "ADMIN_EMAIL"
        | "ADMIN_PASSWORD"
        | "API_BASE_URL"
        | "APP_PATH"
        | "APP_REPOSITORY"
        | "ECOSYSTEM_SYNC_TOKEN"
        | "GITHUB_OWNER"
        | "INPUT_PATH"
        | "INSTALL_OUTCOME"
        | "MANIFEST_PATH"
        | "NODE_PATH"
        | "PREVIEW_URL"
        | "PYTHON_BIN"
        | "REQUIRE_LIVE"
        | "ROOT_VERSION"
        | "SKIPO_APP_ROOT"
        | "SKIPO_INSPECTION_MANIFEST"
        | "SKIPO_OPERATION"
        | "SKIPO_PREVIEW_URL"
        | "SKIPO_PYTHON"
        | "SKIPO_REGISTRY_VERSION"
        | "SKIPO_VERSION"
        | "SYNC_TOKEN"
        | "TAG_VERSION"
        | "FRAMEWORK"
        | "ISSUE_COMMIT_SHA" => VariableClass::InternalTooling,
        _ => VariableClass::Undeclared,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    InvalidAssignment,
    Undeclared,
    NotApplicationConfiguration,
    Duplicate,
    MissingRequired,
    AliasConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub variable: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasChange {
    pub from: String,
    pub to: String,
    pub line: usize,
    pub redundant: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentPlan {
    pub proposed: String,
    pub changes: Vec<AliasChange>,
}

impl EnvironmentPlan {
    pub fn is_noop(&self) -> bool {
        self.changes.is_empty()
    }
}

fn valid_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z'))
        && bytes.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
}

fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start();
    let line = line.strip_prefix("export ").unwrap_or(line);
    let (name, value) = line.split_once('=')?;
    let name = name.trim();
    if valid_name(name) {
        Some((name, value.trim()))
    } else {
        None
    }
}

/// Inspect a UTF-8 .env file without modifying it. Unknown variables,
/// duplicated declarations and conflicting canonical/legacy values fail closed.
/// `require_required` applies to the app's canonical environment template;
/// an optional per-environment override may legitimately omit either variable.
pub fn plan_env_file(
    source: &str,
    require_required: bool,
) -> Result<EnvironmentPlan, Vec<Diagnostic>> {
    let mut declared: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = parse_assignment(raw) else {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InvalidAssignment,
                variable: String::new(),
                line: Some(line_number),
            });
            continue;
        };

        let kind = match classify(name) {
            VariableClass::CanonicalRequired
            | VariableClass::CanonicalOptional
            | VariableClass::DeprecatedAlias(_) => None,
            VariableClass::RuntimePlatform | VariableClass::InternalTooling => {
                Some(DiagnosticKind::NotApplicationConfiguration)
            }
            VariableClass::Undeclared => Some(DiagnosticKind::Undeclared),
        };
        if let Some(kind) = kind {
            diagnostics.push(Diagnostic {
                kind,
                variable: name.to_owned(),
                line: Some(line_number),
            });
        }
        if declared
            .insert(name.to_owned(), (line_number, value.to_owned()))
            .is_some()
        {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Duplicate,
                variable: name.to_owned(),
                line: Some(line_number),
            });
        }
    }

    if require_required {
        for required in REQUIRED {
            let satisfied = declared.contains_key(required)
                || ALIASES
                    .iter()
                    .any(|(alias, target)| *target == required && declared.contains_key(*alias));
            if !satisfied {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::MissingRequired,
                    variable: required.to_owned(),
                    line: None,
                });
            }
        }
    }

    let mut changes = Vec::new();
    for (alias, target) in ALIASES {
        if let Some((line, value)) = declared.get(alias) {
            let redundant = if let Some((_, original)) = declared.get(target) {
                if original != value {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::AliasConflict,
                        variable: alias.to_owned(),
                        line: Some(*line),
                    });
                }
                true
            } else {
                false
            };
            changes.push(AliasChange {
                from: alias.to_owned(),
                to: target.to_owned(),
                line: *line,
                redundant,
            });
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut proposed = String::with_capacity(source.len());
    for (index, text) in source.split_inclusive('\n').enumerate() {
        match changes.iter().find(|change| change.line == index + 1) {
            Some(change) if change.redundant => {}
            Some(change) => {
                let Some((name, _)) = parse_assignment(text) else {
                    unreachable!("preflight validated each changed assignment")
                };
                debug_assert_eq!(name, change.from);
                proposed.push_str(&text.replacen(&change.from, &change.to, 1));
            }
            None => proposed.push_str(text),
        }
    }
    Ok(EnvironmentPlan { proposed, changes })
}

/// Render a credential-free template. Values are placeholders, never secrets.
pub const fn canonical_env_example() -> &'static str {
    "SYSTEM_DATABASE_URL=\nAPP_URL=\n# SYSTEM_DATABASE_URL_2=\n# SYSTEM_DATABASE_URL_3=\n"
}
