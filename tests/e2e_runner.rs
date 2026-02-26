use std::process::Command;

#[test]
fn e2e_tests() {
    // Ensure binary is built (cargo test already compiles, but the bin must exist)
    let e2e_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/e2e");

    // Install npm deps if needed
    if !e2e_dir.join("node_modules").exists() {
        let install = Command::new("npm")
            .arg("install")
            .current_dir(&e2e_dir)
            .output()
            .expect("npm install failed to start");
        assert!(
            install.status.success(),
            "npm install failed:\n{}",
            String::from_utf8_lossy(&install.stderr)
        );
    }

    let output = Command::new("node_modules/.bin/tui-test")
        .arg("--updateSnapshot")
        .current_dir(&e2e_dir)
        .output()
        .expect("tui-test failed to start");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        panic!(
            "E2E tests failed:\nstdout:\n{}\nstderr:\n{}",
            stdout, stderr
        );
    }

    // Print summary so it shows in cargo test output
    eprint!("{}", stderr);
}
