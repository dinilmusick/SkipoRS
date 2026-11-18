//! Compile and run a generated Rust application independently of SkipoRS.
use skipors::{plan_rust_scaffold, verify_env, write_rust_scaffold};
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root =
        std::env::temp_dir().join(format!("skipors-consumer-{}-{suffix}", std::process::id()));
    fs::create_dir(&root)?;
    let app = root.join("native-app");
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let expected = plan_rust_scaffold("native-app")?;
        write_rust_scaffold(&app, "native-app")?;
        for (path, bytes) in expected {
            assert_eq!(fs::read_to_string(app.join(path))?, bytes);
        }
        verify_env(&fs::read_to_string(app.join(".env.example"))?)
            .map_err(|errors| format!("environment rejected: {errors:?}"))?;
        let output = Command::new("cargo")
            .arg("run")
            .arg("--offline")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(app.join("Cargo.toml"))
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "generated Cargo execution failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        assert!(String::from_utf8(output.stdout)?.contains("Configure canonical providers"));
        Ok(())
    })();
    fs::remove_dir_all(&root)?;
    result?;
    println!("SkipoRS generated Rust app consumer: PASS");
    Ok(())
}
