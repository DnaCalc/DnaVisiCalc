#[cfg(target_os = "windows")]
#[test]
fn windows_terminal_sendkeys_harness_runs_when_enabled() {
    use std::path::PathBuf;
    use std::process::Command;

    if std::env::var_os("DNAVISICALC_RUN_WINDOWS_TERMINAL_E2E").is_none() {
        eprintln!(
            "skipping windows terminal e2e harness (set DNAVISICALC_RUN_WINDOWS_TERMINAL_E2E=1)"
        );
        return;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .join("..")
        .join("..")
        .canonicalize()
        .expect("resolve repo root");
    let script = repo_root
        .join("scripts")
        .join("windows")
        .join("repro_double_keypress.ps1");

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script.to_str().expect("script path utf8"),
            "-ProjectRoot",
            repo_root.to_str().expect("repo path utf8"),
            "-NoBuild",
        ])
        .output()
        .expect("run repro script");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "windows terminal harness failed (status={:?})\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        );
    }
}
