use assert_cmd::Command;

#[test]
fn test_from_file() {
    let mut cmd = Command::cargo_bin("torrust-bencode2json").unwrap();
    cmd.arg("-i")
        .arg("tests/fixtures/sample.bencode")
        .arg("-o")
        .arg("output.json")
        .assert()
        .success();

    // todo: check contents
    // Read the file. It should contain: ["spam"]
}

#[test]
fn test_stdin_stdout() {
    let mut cmd = Command::cargo_bin("torrust-bencode2json").unwrap();
    cmd.write_stdin("4:spam")
        .assert()
        .success()
        .stdout("\"spam\"\n");
}
