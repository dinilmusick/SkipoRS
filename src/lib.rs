//! Portable Skipo environment governance and deterministic Rust app scaffold primitives.
//! No agent scheduling, backend authority, or platform-specific service is implied.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const REQUIRED: [&str; 2] = ["SYSTEM_DATABASE_URL", "APP_URL"];
pub const OPTIONAL: [&str; 2] = ["SYSTEM_DATABASE_URL_2", "SYSTEM_DATABASE_URL_3"];
pub const LEGACY_APP_URL: &str = "DEPM_GATEWAY_DOMAIN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    InvalidSlug,
    InvalidDeclaration(usize),
    Duplicate(String),
    Unknown(String),
    Missing(String),
    Conflict(String),
    UnsafePath,
    Io(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ContractError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvPlan {
    pub actions: Vec<String>,
    pub repaired: String,
}

fn valid_name(key: &str) -> bool {
    let mut chars = key.bytes();
    matches!(chars.next(), Some(b'A'..=b'Z'))
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
}

fn allowed(key: &str) -> bool {
    REQUIRED.contains(&key) || OPTIONAL.contains(&key)
}

/// Plan only recognized .env declarations. Unknown keys and duplicate/colliding
/// aliases reject the entire plan without mutating the original input.
pub fn plan_env(source: &str) -> Result<EnvPlan, Vec<ContractError>> {
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut occurrences: Vec<(usize, String)> = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let declaration = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((name, _value)) = declaration.split_once('=') else {
            errors.push(ContractError::InvalidDeclaration(index + 1));
            continue;
        };
        let name = name.trim();
        if !valid_name(name) {
            errors.push(ContractError::InvalidDeclaration(index + 1));
        } else if !seen.insert(name.to_owned()) {
            errors.push(ContractError::Duplicate(name.to_owned()));
        } else if !allowed(name) && name != LEGACY_APP_URL {
            errors.push(ContractError::Unknown(name.to_owned()));
        }
        occurrences.push((index, name.to_owned()));
    }
    if seen.contains(LEGACY_APP_URL) && seen.contains("APP_URL") {
        errors.push(ContractError::Conflict(LEGACY_APP_URL.to_owned()));
    }
    for required in REQUIRED {
        if !seen.contains(required) && !(required == "APP_URL" && seen.contains(LEGACY_APP_URL)) {
            errors.push(ContractError::Missing(required.to_owned()));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut rewritten = String::new();
    for line in source.split_inclusive('\n') {
        let (body, newline) = match line.strip_suffix('\n') {
            Some(value) => (value, "\n"),
            None => (line, ""),
        };
        let replaced = if seen.contains(LEGACY_APP_URL) {
            let prefix_len = body.len() - body.trim_start().len();
            let tail = &body[prefix_len..];
            let after_export = tail.strip_prefix("export ").unwrap_or(tail);
            if let Some((key, _value)) = after_export.split_once('=') {
                if key.trim() == LEGACY_APP_URL {
                    let key_start = body.find(LEGACY_APP_URL).unwrap_or(0);
                    format!(
                        "{}APP_URL{}",
                        &body[..key_start],
                        &body[key_start + LEGACY_APP_URL.len()..]
                    )
                } else {
                    format!("{body}{newline}")
                }
            } else {
                format!("{body}{newline}")
            }
        } else {
            format!("{body}{newline}")
        };
        if replaced.ends_with('\n') || newline.is_empty() {
            rewritten.push_str(&replaced);
        } else {
            rewritten.push_str(&replaced);
            rewritten.push_str(newline);
        }
    }
    let actions = if seen.contains(LEGACY_APP_URL) {
        vec!["rename DEPM_GATEWAY_DOMAIN to APP_URL".to_owned()]
    } else {
        Vec::new()
    };
    Ok(EnvPlan {
        actions,
        repaired: rewritten,
    })
}

pub fn verify_env(source: &str) -> Result<(), Vec<ContractError>> {
    let plan = plan_env(source)?;
    if plan.actions.is_empty() {
        Ok(())
    } else {
        Err(vec![ContractError::Unknown(LEGACY_APP_URL.to_owned())])
    }
}

pub fn valid_slug(slug: &str) -> bool {
    let bytes = slug.as_bytes();
    (1..=63).contains(&bytes.len())
        && bytes.first().is_some_and(u8::is_ascii_lowercase)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// Return exact deterministic scaffold bytes without fetching dependencies or
/// inventing working authentication/storage. A consuming app wires its providers.
pub fn plan_rust_scaffold(slug: &str) -> Result<BTreeMap<String, String>, ContractError> {
    if !valid_slug(slug) {
        return Err(ContractError::InvalidSlug);
    }
    let cargo = format!(
        "[package]\nname = \"{slug}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
    );
    let source = "fn main() {\n    println!(\"Configure canonical providers before running application services.\");\n}\n";
    let env =
        "SYSTEM_DATABASE_URL=postgres://configure-at-runtime\nAPP_URL=http://localhost:3000\n";
    let readme = format!(
        "# {slug}\n\nGenerated by SkipoRS. Replace example environment values. \
         Canonical auth, workspaces and storage must be wired from approved providers; \
         this scaffold is not a deployed application.\n"
    );
    Ok(BTreeMap::from([
        ("Cargo.toml".to_owned(), cargo),
        (".env.example".to_owned(), env.to_owned()),
        ("README.md".to_owned(), readme),
        ("src/main.rs".to_owned(), source.to_owned()),
    ]))
}

/// Create only a genuinely new project directory; never overwrite existing
/// files or follow a project-directory symlink.
pub fn write_rust_scaffold(root: &Path, slug: &str) -> Result<(), ContractError> {
    let files = plan_rust_scaffold(slug)?;
    let parent = root.parent().ok_or(ContractError::UnsafePath)?;
    if !root.is_absolute()
        || root.file_name().and_then(|name| name.to_str()) != Some(slug)
        || root
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        || !parent.is_dir()
        || parent.is_symlink()
        || root.exists()
        || root.is_symlink()
    {
        return Err(ContractError::UnsafePath);
    }
    std::fs::create_dir(root).map_err(|error| ContractError::Io(error.to_string()))?;
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir(root.join("src"))?;
        for (rel, content) in files {
            std::fs::write(root.join(rel), content)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(root);
        return Err(ContractError::Io(error.to_string()));
    }
    Ok(())
}
