use std::process::Command;

#[test]
fn invalid_model_configuration_fails_before_announcing_readiness() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("invalid.toml");
    std::fs::write(&config, "threads = 0").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_substream"))
        .args(["stream", "--config"])
        .arg(config)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "a failed model must not announce readiness"
    );
    assert!(
        !output.stderr.is_empty(),
        "the user needs an actionable failure"
    );
}
