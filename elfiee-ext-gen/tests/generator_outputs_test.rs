use elfiee_ext_gen::core::generator::Generator;
use elfiee_ext_gen::models::config::ExtensionConfig;
use std::path::PathBuf;

#[test]
fn test_generate_extension_produces_expected_structure() {
    let generator = Generator::new().expect("generator should initialize with bundled templates");
    let config = ExtensionConfig::new(
        "directory_inspector",
        "directory",
        vec!["scan".into(), "trust".into()],
    );

    let generated = generator
        .generate_extension(&config)
        .expect("generator should render templates without errors");

    let mod_path = PathBuf::from("src-tauri/src/extensions/directory_inspector/mod.rs");
    let scan_cap_path =
        PathBuf::from("src-tauri/src/extensions/directory_inspector/directory_inspector_scan.rs");
    let trust_cap_path =
        PathBuf::from("src-tauri/src/extensions/directory_inspector/directory_inspector_trust.rs");
    let tests_path = PathBuf::from("src-tauri/src/extensions/directory_inspector/tests.rs");

    assert!(
        generated.files.contains_key(&mod_path),
        "mod.rs should be generated for the extension"
    );
    assert!(
        generated.files.contains_key(&scan_cap_path),
        "scan capability file should be generated"
    );
    assert!(
        generated.files.contains_key(&trust_cap_path),
        "trust capability file should be generated"
    );
    assert!(
        generated.files.contains_key(&tests_path),
        "tests.rs should be generated for the extension"
    );

    let mod_rs = &generated.files[&mod_path];
    assert!(
        mod_rs.contains("pub struct DirectoryInspectorScanPayload"),
        "mod.rs should declare DirectoryInspectorScanPayload, got:\n{mod_rs}"
    );
    assert!(
        mod_rs.contains("pub struct DirectoryInspectorTrustPayload"),
        "mod.rs should declare DirectoryInspectorTrustPayload, got:\n{mod_rs}"
    );

    let scan_rs = &generated.files[&scan_cap_path];
    assert!(
        scan_rs.contains(r#"todo!("Deserialize DirectoryInspectorScanPayload from cmd.payload")"#),
        "scan capability file should include the TODO marker with payload name, got:\n{scan_rs}"
    );

    let tests_rs = &generated.files[&tests_path];
    assert!(
        tests_rs.contains("test_scan_basic"),
        "tests.rs should include the scan capability basic test, got:\n{tests_rs}"
    );
    assert!(
        tests_rs.contains("test_trust_basic"),
        "tests.rs should include the trust capability basic test, got:\n{tests_rs}"
    );
}

#[test]
fn test_generate_extension_returns_next_steps() {
    let generator = Generator::new().expect("generator should initialize with bundled templates");
    let config = ExtensionConfig::new("markdown", "markdown", vec!["write".into(), "read".into()]);

    let generated = generator
        .generate_extension(&config)
        .expect("generator should render templates without errors");

    assert!(
        !generated.next_steps.is_empty(),
        "generator should provide next steps to guide developers"
    );
    assert!(
        generated
            .next_steps
            .iter()
            .any(|step| step.contains("cargo test markdown::tests")),
        "next steps should include running the generated test suite"
    );
}
