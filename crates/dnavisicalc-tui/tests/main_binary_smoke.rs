use assert_cmd::Command;

#[test]
fn binary_can_start_in_test_exit_mode() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("dnavisicalc"));
    cmd.env("DNAVISICALC_TEST_EXIT", "1");
    cmd.assert().success();
}
