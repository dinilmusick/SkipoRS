use skipors::{
    plan_env, plan_rust_scaffold, valid_slug, verify_env, write_rust_scaffold, ContractError,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn canonical_environment_is_strict_and_repair_is_idempotent() {
    let input = "# example\nexport SYSTEM_DATABASE_URL=postgres://db\nDEPM_GATEWAY_DOMAIN=https://app.example\n";
    let planned = plan_env(input).unwrap();
    assert_eq!(
        planned.actions,
        vec!["rename DEPM_GATEWAY_DOMAIN to APP_URL"]
    );
    assert!(planned.repaired.contains("APP_URL=https://app.example"));
    assert!(!planned.repaired.contains("DEPM_GATEWAY_DOMAIN"));
    assert!(verify_env(&planned.repaired).is_ok());
    let second = plan_env(&planned.repaired).unwrap();
    assert!(second.actions.is_empty());
    assert_eq!(second.repaired, planned.repaired);
}

#[test]
fn environment_conflicts_duplicates_and_unknowns_fail_closed() {
    let cases = [
        ("APP_URL=https://app\n", "missing database"),
        ("SYSTEM_DATABASE_URL=db\n", "missing app"),
        (
            "SYSTEM_DATABASE_URL=db\nAPP_URL=a\nAPP_URL=b\n",
            "duplicate",
        ),
        (
            "SYSTEM_DATABASE_URL=db\nAPP_URL=a\nDEPM_GATEWAY_DOMAIN=b\n",
            "conflicting alias",
        ),
        (
            "SYSTEM_DATABASE_URL=db\nAPP_URL=a\nUNDECLARED_SECRET=abc\n",
            "unknown application setting",
        ),
        (
            "SYSTEM_DATABASE_URL=db\nAPP_URL=a\nmalformed variable\n",
            "malformed syntax",
        ),
        (
            "SYSTEM_DATABASE_URL=db\nAPP_URL=a\nNODE_ENV=production\n",
            "platform setting in app env",
        ),
    ];
    for (source, reason) in cases {
        assert!(plan_env(source).is_err(), "{reason}");
    }
    let legacy = "SYSTEM_DATABASE_URL=db\nDEPM_GATEWAY_DOMAIN=a\n";
    assert!(matches!(
        verify_env(legacy),
        Err(errors) if errors.contains(&ContractError::Unknown("DEPM_GATEWAY_DOMAIN".to_owned()))
    ));
    assert!(plan_env(
        "SYSTEM_DATABASE_URL=db\nAPP_URL=a\nSYSTEM_DATABASE_URL_2=db2\nSYSTEM_DATABASE_URL_3=db3\n"
    )
    .is_ok());
}

#[test]
fn deterministic_scaffold_rejects_unsafe_names_and_is_never_blank() {
    for invalid in [
        "",
        "-app",
        "app-",
        "Foo",
        "../escape",
        "hello/world",
        "hi there",
        "a_b",
    ] {
        assert!(!valid_slug(invalid));
        assert_eq!(plan_rust_scaffold(invalid), Err(ContractError::InvalidSlug));
    }
    let first = plan_rust_scaffold("native-app").unwrap();
    let second = plan_rust_scaffold("native-app").unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 4);
    assert!(verify_env(&first[".env.example"]).is_ok());
    assert!(first["Cargo.toml"].contains("name = \"native-app\""));
    assert!(first["README.md"].contains("not a deployed application"));
}

fn test_root() -> PathBuf {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    std::env::temp_dir().join(format!(
        "skipors-test-{}-{}",
        std::process::id(),
        now.as_nanos()
    ))
}

#[test]
fn scaffold_writes_new_projects_without_overwriting_existing_data() {
    let root = test_root();
    fs::create_dir(&root).unwrap();
    let app = root.join("native-app");
    write_rust_scaffold(&app, "native-app").unwrap();
    let original = fs::read_to_string(app.join("Cargo.toml")).unwrap();
    assert_eq!(
        write_rust_scaffold(&app, "native-app"),
        Err(ContractError::UnsafePath)
    );
    assert_eq!(
        fs::read_to_string(app.join("Cargo.toml")).unwrap(),
        original
    );
    assert!(app.join("src/main.rs").is_file());
    assert_eq!(
        write_rust_scaffold(&root.join("different"), "native-app"),
        Err(ContractError::UnsafePath)
    );
    assert_eq!(
        write_rust_scaffold(&PathBuf::from("relative-app"), "relative-app"),
        Err(ContractError::UnsafePath)
    );
    assert_eq!(
        write_rust_scaffold(&root.join("../unsafe"), "../unsafe"),
        Err(ContractError::InvalidSlug)
    );
    fs::remove_dir_all(&root).unwrap();
}
