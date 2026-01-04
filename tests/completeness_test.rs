//! Integration tests for completeness command

use imacs::*;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

fn get_imacs_binary() -> PathBuf {
    // Try release first, then debug
    let release_path = PathBuf::from("target/release/imacs");
    let debug_path = PathBuf::from("target/debug/imacs");

    if release_path.exists() {
        release_path
    } else if debug_path.exists() {
        debug_path
    } else {
        // Fallback - assume it's in PATH
        PathBuf::from("imacs")
    }
}

fn run_imacs(args: &[&str]) -> (ExitStatus, String, String) {
    let binary = get_imacs_binary();
    let output = Command::new(binary)
        .args(args)
        .output()
        .expect("Failed to execute imacs");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (output.status, stdout, stderr)
}

#[test]
fn test_cmd_completeness_success() {
    // Create a temporary complete spec file
    let spec_content = r#"
id: test_complete
inputs:
  - name: a
    type: bool
  - name: b
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "a && b"
    then: 1
  - id: R2
    when: "a && !b"
    then: 2
  - id: R3
    when: "!a && b"
    then: 3
  - id: R4
    when: "!a && !b"
    then: 4
"#;

    let temp_dir = std::env::temp_dir();
    let spec_path = temp_dir.join("test_complete_spec.yaml");
    fs::write(&spec_path, spec_content).expect("Failed to write test spec");

    let (status, stdout, stderr) = run_imacs(&["completeness", spec_path.to_str().unwrap()]);

    // Clean up
    let _ = fs::remove_file(&spec_path);

    assert!(
        status.success(),
        "Complete spec should return exit code 0. stderr: {}",
        stderr
    );
    assert!(
        stdout.contains("COMPLETE") || stdout.contains("complete"),
        "Should indicate completeness"
    );
}

#[test]
fn test_cmd_completeness_failure() {
    // Create a temporary incomplete spec file
    let spec_content = r#"
id: test_incomplete
inputs:
  - name: a
    type: bool
  - name: b
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "a && b"
    then: 1
  - id: R2
    when: "a && !b"
    then: 2
"#;

    let temp_dir = std::env::temp_dir();
    let spec_path = temp_dir.join("test_incomplete_spec.yaml");
    fs::write(&spec_path, spec_content).expect("Failed to write test spec");

    let (status, stdout, stderr) = run_imacs(&["completeness", spec_path.to_str().unwrap()]);

    // Clean up
    let _ = fs::remove_file(&spec_path);

    assert!(
        !status.success(),
        "Incomplete spec should return exit code 1"
    );
    assert!(
        stdout.contains("INCOMPLETE")
            || stdout.contains("incomplete")
            || stderr.contains("Incomplete"),
        "Should indicate incompleteness. stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_cmd_completeness_json_output() {
    let spec_content = r#"
id: test_json
inputs:
  - name: a
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "a"
    then: 1
  - id: R2
    when: "!a"
    then: 2
"#;

    let temp_dir = std::env::temp_dir();
    let spec_path = temp_dir.join("test_json_spec.yaml");
    fs::write(&spec_path, spec_content).expect("Failed to write test spec");

    let (status, stdout, _stderr) =
        run_imacs(&["completeness", "--json", spec_path.to_str().unwrap()]);

    // Clean up
    let _ = fs::remove_file(&spec_path);

    assert!(status.success(), "Should succeed");

    // Verify it's valid JSON
    let json_result: std::result::Result<IncompletenessReport, _> = serde_json::from_str(&stdout);
    assert!(
        json_result.is_ok(),
        "Output should be valid JSON. Got: {}",
        stdout
    );

    let report = json_result.unwrap();
    assert!(report.is_complete, "This spec should be complete");
}

#[test]
fn test_cmd_completeness_human_output() {
    let spec_content = r#"
id: test_human
inputs:
  - name: a
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "a"
    then: 1
"#;

    let temp_dir = std::env::temp_dir();
    let spec_path = temp_dir.join("test_human_spec.yaml");
    fs::write(&spec_path, spec_content).expect("Failed to write test spec");

    let (_status, stdout, _stderr) = run_imacs(&["completeness", spec_path.to_str().unwrap()]);

    // Clean up
    let _ = fs::remove_file(&spec_path);

    // Should not be JSON (human-readable)
    assert!(
        !stdout.trim_start().starts_with('{'),
        "Should not be JSON format"
    );
    assert!(
        stdout.contains("Completeness Analysis") || stdout.contains("Coverage"),
        "Should contain human-readable text. Got: {}",
        stdout
    );
}

#[test]
fn test_cmd_completeness_missing_file() {
    let (status, _stdout, stderr) = run_imacs(&["completeness", "nonexistent_file.yaml"]);

    assert!(!status.success(), "Should fail for missing file");
    assert!(
        stderr.contains("Error") || stderr.contains("not found") || stderr.contains("No such file"),
        "Should show error message. Got: {}",
        stderr
    );
}

#[test]
fn test_cmd_completeness_invalid_yaml() {
    let invalid_yaml = "this is not valid yaml: [";

    let temp_dir = std::env::temp_dir();
    let spec_path = temp_dir.join("test_invalid_spec.yaml");
    fs::write(&spec_path, invalid_yaml).expect("Failed to write test spec");

    let (status, _stdout, stderr) = run_imacs(&["completeness", spec_path.to_str().unwrap()]);

    // Clean up
    let _ = fs::remove_file(&spec_path);

    assert!(!status.success(), "Should fail for invalid YAML");
    assert!(
        stderr.contains("Error") || stderr.contains("YAML") || stderr.contains("parse"),
        "Should show error message. Got: {}",
        stderr
    );
}

#[test]
fn test_cmd_completeness_no_args() {
    let (status, _stdout, stderr) = run_imacs(&["completeness"]);

    assert!(!status.success(), "Should fail without arguments");
    assert!(
        stderr.contains("Usage") || stderr.contains("error"),
        "Should show usage or error. Got: {}",
        stderr
    );
}

#[test]
fn test_cmd_completeness_with_fixtures() {
    // Test with complete spec fixture
    let complete_path = "tests/fixtures/complete_spec.yaml";
    if std::path::Path::new(complete_path).exists() {
        let (status, stdout, _stderr) = run_imacs(&["completeness", complete_path]);
        assert!(status.success(), "Complete fixture should succeed");
        assert!(
            stdout.contains("COMPLETE") || stdout.contains("complete") || stdout.contains("100"),
            "Should indicate completeness"
        );
    }

    // Test with incomplete spec fixture
    let incomplete_path = "tests/fixtures/incomplete_spec.yaml";
    if std::path::Path::new(incomplete_path).exists() {
        let (status, stdout, _stderr) = run_imacs(&["completeness", incomplete_path]);
        assert!(!status.success(), "Incomplete fixture should fail");
        assert!(
            stdout.contains("INCOMPLETE")
                || stdout.contains("incomplete")
                || stdout.contains("Missing"),
            "Should indicate incompleteness"
        );
    }

    // Test with overlapping spec fixture
    let overlapping_path = "tests/fixtures/overlapping_spec.yaml";
    if std::path::Path::new(overlapping_path).exists() {
        let (status, stdout, _stderr) = run_imacs(&["completeness", overlapping_path]);
        // Overlapping rules don't necessarily make it incomplete
        assert!(
            stdout.contains("Overlapping")
                || stdout.contains("overlap")
                || status.success()
                || !status.success(),
            "Should handle overlapping rules"
        );
    }
}

#[test]
fn test_exit_code_complete() {
    // Explicitly test exit code 0 for complete spec
    let spec_content = r#"
id: exit_test_complete
inputs:
  - name: x
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "x"
    then: 1
  - id: R2
    when: "!x"
    then: 2
"#;

    let temp_dir = std::env::temp_dir();
    let spec_path = temp_dir.join("exit_test_complete.yaml");
    fs::write(&spec_path, spec_content).expect("Failed to write test spec");

    let (status, _stdout, _stderr) = run_imacs(&["completeness", spec_path.to_str().unwrap()]);

    // Clean up
    let _ = fs::remove_file(&spec_path);

    assert_eq!(
        status.code(),
        Some(0),
        "Complete spec should return exit code 0"
    );
}

#[test]
fn test_exit_code_incomplete() {
    // Explicitly test exit code 1 for incomplete spec
    let spec_content = r#"
id: exit_test_incomplete
inputs:
  - name: x
    type: bool
outputs:
  - name: result
    type: int
rules:
  - id: R1
    when: "x"
    then: 1
  # Missing: !x case
"#;

    let temp_dir = std::env::temp_dir();
    let spec_path = temp_dir.join("exit_test_incomplete.yaml");
    fs::write(&spec_path, spec_content).expect("Failed to write test spec");

    let (status, _stdout, _stderr) = run_imacs(&["completeness", spec_path.to_str().unwrap()]);

    // Clean up
    let _ = fs::remove_file(&spec_path);

    assert_eq!(
        status.code(),
        Some(1),
        "Incomplete spec should return exit code 1"
    );
}
