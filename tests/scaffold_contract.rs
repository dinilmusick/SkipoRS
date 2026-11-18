use skipo_rs::environment::plan_env_file;
use skipo_rs::scaffold::{
    plan_rust_scaffold, valid_slug, write_rust_scaffold, ScaffoldError,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
        assert_eq!(plan_rust_scaffold(invalid), Err(ScaffoldError::InvalidSlug));
    }
    let first = plan_rust_scaffold("native-app").unwrap();
    let second = plan_rust_scaffold("native-app").unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 4);
    assert!(plan_env_file(&first[".env.example"], true).is_ok());
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
        Err(ScaffoldError::UnsafePath)
    );
    assert_eq!(
        fs::read_to_string(app.join("Cargo.toml")).unwrap(),
        original
    );
    assert!(app.join("src/main.rs").is_file());
    assert_eq!(
        write_rust_scaffold(&root.join("different"), "native-app"),
        Err(ScaffoldError::UnsafePath)
    );
    assert_eq!(
        write_rust_scaffold(&PathBuf::from("relative-app"), "relative-app"),
        Err(ScaffoldError::UnsafePath)
    );
    assert_eq!(
        write_rust_scaffold(&root.join("../unsafe"), "../unsafe"),
        Err(ScaffoldError::InvalidSlug)
    );
    fs::remove_dir_all(&root).unwrap();
}
