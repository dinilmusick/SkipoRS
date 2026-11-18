use skipo_rs::environment::{
    canonical_env_example, classify, plan_env_file, DiagnosticKind, VariableClass, ALIASES,
    OPTIONAL, REQUIRED,
};

fn has_kind(errs: &[skipo_rs::environment::Diagnostic], kind: DiagnosticKind) -> bool {
    errs.iter().any(|err| err.kind == kind)
}

#[test]
fn pinned_upstream_fixture_matches_exact_canonical_name_inventory() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/skipo_environment_contract.json")).unwrap();
    assert_eq!(source["schema_version"], 2);
    assert_eq!(source["contract_version"], "2.0.0");
    let required: Vec<_> = source["canonical"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    let optional: Vec<_> = source["canonical"]["optional"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert_eq!(required, REQUIRED);
    assert_eq!(optional, OPTIONAL);

    for (alias, target) in ALIASES {
        assert_eq!(source["deprecated_aliases"][alias]["target"], target);
        assert_eq!(classify(alias), VariableClass::DeprecatedAlias(target));
    }

    for name in source["runtime_platform"].as_object().unwrap().keys() {
        assert_eq!(classify(name), VariableClass::RuntimePlatform);
    }
    for name in source["internal_tooling"].as_object().unwrap().keys() {
        assert_eq!(classify(name), VariableClass::InternalTooling);
    }
}

#[test]
fn canonical_template_has_no_credentials_and_is_valid() {
    let template = canonical_env_example();
    let plan = plan_env_file(template, true).unwrap();
    assert!(plan.is_noop());
    assert_eq!(plan.proposed, template);
    assert!(!template.contains("postgres://"));
}

#[test]
fn canonical_and_optional_app_variables_are_accepted() {
    let input = "SYSTEM_DATABASE_URL=postgres://db\nAPP_URL=https://app.example\nSYSTEM_DATABASE_URL_2=postgres://backup\n";
    assert!(plan_env_file(input, true).unwrap().is_noop());
}

#[test]
fn deprecated_alias_preserves_values_and_export_prefix() {
    let input = "export SYSTEM_DATABASE_URL=postgres://db\n  export DEPM_GATEWAY_DOMAIN = https://app.example/path?q=1#anchor\n";
    let plan = plan_env_file(input, true).unwrap();
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].to, "APP_URL");
    assert!(plan.proposed.contains("  export APP_URL = https://app.example/path?q=1#anchor"));
    assert!(!plan.proposed.contains("DEPM_GATEWAY_DOMAIN"));
    assert!(plan_env_file(&plan.proposed, true).unwrap().is_noop());
}

#[test]
fn primary_database_alias_is_migrated_without_touching_connection_string() {
    let input = "SYSTEM_DATABASE_URL_1=postgres://user:pass@localhost:5432/db?sslmode=require\nAPP_URL=https://x.test\n";
    let output = plan_env_file(input, true).unwrap().proposed;
    assert!(output.contains("SYSTEM_DATABASE_URL=postgres://user:pass@localhost:5432/db?sslmode=require"));
    assert!(!output.contains("SYSTEM_DATABASE_URL_1="));
}

#[test]
fn mismatched_alias_and_canonical_fail_without_leaking_values() {
    let errors = plan_env_file(
        "SYSTEM_DATABASE_URL=postgres://db\nDEPM_GATEWAY_DOMAIN=https://private.old\nAPP_URL=https://private.new\n",
        true,
    )
    .unwrap_err();
    assert!(has_kind(&errors, DiagnosticKind::AliasConflict));
    assert!(!format!("{errors:?}").contains("private.old"));
    assert!(!format!("{errors:?}").contains("private.new"));
}

#[test]
fn redundant_equal_alias_is_safely_retired_and_idempotent() {
    let input = "APP_URL=https://same.example\nDEPM_GATEWAY_DOMAIN=https://same.example\nSYSTEM_DATABASE_URL=postgres://db\n";
    let plan = plan_env_file(input, true).unwrap();
    assert_eq!(plan.changes.len(), 1);
    assert!(plan.changes[0].redundant);
    assert_eq!(plan.proposed.matches("APP_URL=").count(), 1);
    assert!(!plan.proposed.contains("DEPM_GATEWAY_DOMAIN"));
    assert!(plan_env_file(&plan.proposed, true).unwrap().is_noop());
}

#[test]
fn missing_required_variables_fail_closed() {
    let errors = plan_env_file("APP_URL=https://example.test\n", true).unwrap_err();
    assert!(has_kind(&errors, DiagnosticKind::MissingRequired));
    assert!(plan_env_file("APP_URL=https://example.test\n", false).is_ok());
}

#[test]
fn undeclared_app_secret_does_not_get_normalized_or_accepted() {
    let errors = plan_env_file(
        "SYSTEM_DATABASE_URL=postgres://db\nAPP_URL=https://example.test\nMY_HIDDEN_TOKEN=secret\n",
        true,
    )
    .unwrap_err();
    assert!(has_kind(&errors, DiagnosticKind::Undeclared));
    assert!(!format!("{errors:?}").contains("secret"));
}

#[test]
fn platform_and_tooling_variables_are_not_app_configuration() {
    for invalid in ["NODE_ENV", "CI", "SKIPO_VERSION", "ADMIN_PASSWORD"] {
        let input = format!("SYSTEM_DATABASE_URL=postgres://db\nAPP_URL=https://example.test\n{invalid}=set\n");
        let errors = plan_env_file(&input, true).unwrap_err();
        assert!(has_kind(&errors, DiagnosticKind::NotApplicationConfiguration));
    }
}

#[test]
fn duplicate_and_invalid_assignments_fail_closed() {
    let errors = plan_env_file("APP_URL=a\nAPP_URL=b\nSYSTEM_DATABASE_URL=db\n", true)
        .unwrap_err();
    assert!(has_kind(&errors, DiagnosticKind::Duplicate));
    let errors = plan_env_file("APP_URL=https://x.test\nSYSTEM_DATABASE_URL=db\nlowercase_key=x\n", true)
        .unwrap_err();
    assert!(has_kind(&errors, DiagnosticKind::InvalidAssignment));
}

#[test]
fn preserve_comments_unrelated_lines_and_crlf() {
    let input = "# Keep this comment\r\nSYSTEM_DATABASE_URL=postgres://db\r\nDEPM_GATEWAY_DOMAIN=https://app.test\r\n\r\n";
    let plan = plan_env_file(input, true).unwrap();
    assert_eq!(
        plan.proposed,
        "# Keep this comment\r\nSYSTEM_DATABASE_URL=postgres://db\r\nAPP_URL=https://app.test\r\n\r\n"
    );
}
