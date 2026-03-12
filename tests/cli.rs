use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str as pred;

// Two TLP hex strings used across multiple tests
const CONF_READ: &str = "04000001 0000220f 01070000 9eece789";
const CPL_DATA: &str  = "4a000001 2001FF00 C281FF10 00000000";

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("rtlp-tool").unwrap()
}

// ── Basic parsing ─────────────────────────────────────────────────────────────

#[test]
fn single_tlp_conf_read() {
    cmd()
        .args(["-i", CONF_READ])
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"))
        .stdout(pred::contains("3DW no Data Header"))
        .stdout(pred::contains("Bus"))
        .stdout(pred::contains("Reg Nr"));
}

#[test]
fn single_tlp_cpl_data() {
    cmd()
        .args(["-i", CPL_DATA])
        .assert()
        .success()
        .stdout(pred::contains("CplData"))
        .stdout(pred::contains("3DW with Data Header"))
        .stdout(pred::contains("Compl ID"))
        .stdout(pred::contains("Lower Address"));
}

#[test]
fn multiple_tlps_prints_separators() {
    cmd()
        .args(["-i", CONF_READ, "-i", CPL_DATA])
        .assert()
        .success()
        .stdout(pred::contains("=== TLP #1 ==="))
        .stdout(pred::contains("=== TLP #2 ==="))
        .stdout(pred::contains("ConfType0ReadReq"))
        .stdout(pred::contains("CplData"));
}

#[test]
fn count_limits_number_of_tlps_processed() {
    cmd()
        .args(["-i", CONF_READ, "-i", CPL_DATA, "--count", "1"])
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"))
        // CplData is second — must not appear
        .stdout(pred::contains("CplData").not());
}

// ── stdin input ───────────────────────────────────────────────────────────────

#[test]
fn stdin_single_tlp() {
    cmd()
        .write_stdin(format!("{}\n", CONF_READ))
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"));
}

#[test]
fn stdin_multiple_tlps() {
    cmd()
        .write_stdin(format!("{}\n{}\n", CONF_READ, CPL_DATA))
        .assert()
        .success()
        .stdout(pred::contains("=== TLP #1 ==="))
        .stdout(pred::contains("=== TLP #2 ==="));
}

// ── file input ────────────────────────────────────────────────────────────────

#[test]
fn file_input_two_tlps() {
    cmd()
        .args(["-f", "tests/fixtures/valid_tlps.txt"])
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"))
        .stdout(pred::contains("CplData"))
        .stdout(pred::contains("=== TLP #1 ==="))
        .stdout(pred::contains("=== TLP #2 ==="));
}

#[test]
fn file_not_found_exits_nonzero() {
    cmd()
        .args(["-f", "tests/fixtures/does_not_exist.txt"])
        .assert()
        .failure()
        .stderr(pred::contains("cannot open"));
}

// ── output formats ────────────────────────────────────────────────────────────

#[test]
fn json_output_has_expected_fields() {
    cmd()
        .args(["-i", CPL_DATA, "--output", "json"])
        .assert()
        .success()
        .stdout(pred::contains("\"index\":1"))
        .stdout(pred::contains("\"tlp_type\":\"CplData\""))
        .stdout(pred::contains("\"tlp_format\":\"3DW with Data Header\""))
        .stdout(pred::contains("\"header\":{"))
        .stdout(pred::contains("\"body\":{"))
        .stdout(pred::contains("\"Compl ID\""));
}

#[test]
fn json_multiple_tlps_is_ndjson() {
    let output = cmd()
        .args(["-i", CONF_READ, "-i", CPL_DATA, "--output", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected exactly 2 JSON lines (ndjson), got:\n{}", text);
    for line in &lines {
        assert!(
            line.starts_with('{') && line.ends_with('}'),
            "not a JSON object: {}",
            line
        );
    }
}

#[test]
fn csv_output_starts_with_header_row() {
    cmd()
        .args(["-i", CONF_READ, "--output", "csv"])
        .assert()
        .success()
        .stdout(pred::starts_with(
            "index,source,tlp_type,tlp_format,section,key,value\n",
        ));
}

#[test]
fn csv_output_has_header_and_body_sections() {
    cmd()
        .args(["-i", CONF_READ, "--output", "csv"])
        .assert()
        .success()
        .stdout(pred::contains(",header,Fmt,"))
        .stdout(pred::contains(",header,Length,"))
        .stdout(pred::contains(",body,Reg Nr,"));
}

#[test]
fn csv_multiple_tlps_index_increments() {
    let output = cmd()
        .args(["-i", CONF_READ, "-i", CPL_DATA, "--output", "csv"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("\n1,"), "expected rows with index 1");
    assert!(text.contains("\n2,"), "expected rows with index 2");
}

// ── AER scanning ──────────────────────────────────────────────────────────────

#[test]
fn aer_extracts_tlp_headers_from_dmesg() {
    cmd()
        .args(["--aer", "-f", "tests/fixtures/aer_output.txt"])
        .assert()
        .success()
        .stdout(pred::contains("=== TLP #1 ==="))
        .stdout(pred::contains("=== TLP #2 ==="))
        .stdout(pred::contains("ConfType0ReadReq"));
}

#[test]
fn aer_no_match_exits_nonzero_with_hint() {
    cmd()
        .args(["--aer"])
        .write_stdin("this line has no TLP header in it\n")
        .assert()
        .failure()
        .stderr(pred::contains("no TLP headers found"));
}

// ── lspci scanning ────────────────────────────────────────────────────────────

#[test]
fn lspci_skips_zero_headers_and_parses_nonzero() {
    cmd()
        .args(["--lspci", "-f", "tests/fixtures/lspci_output.txt"])
        .assert()
        .success()
        // 0000:00:1d.0 has an all-zero HeaderLog — must be absent from output
        .stdout(pred::contains("00:1d.0").not())
        // Two non-zero entries must appear
        .stdout(pred::contains("=== TLP #1 ==="))
        .stdout(pred::contains("=== TLP #2 ==="));
}

#[test]
fn lspci_annotates_tlp_with_source_device() {
    cmd()
        .args(["--lspci", "-f", "tests/fixtures/lspci_output.txt"])
        .assert()
        .success()
        .stdout(pred::contains("01:00.0"))
        .stdout(pred::contains("Phison"))
        .stdout(pred::contains("0000:40:00.0"));
}

#[test]
fn lspci_parses_correct_tlp_types() {
    cmd()
        .args(["--lspci", "-f", "tests/fixtures/lspci_output.txt"])
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"))
        .stdout(pred::contains("CplData"));
}

#[test]
fn lspci_no_nonzero_headers_exits_nonzero() {
    cmd()
        .args(["--lspci"])
        .write_stdin("\t\tHeaderLog: 00000000 00000000 00000000 00000000\n")
        .assert()
        .failure()
        .stderr(pred::contains("no non-zero HeaderLog"));
}

#[test]
fn lspci_json_output_includes_source_field() {
    cmd()
        .args(["--lspci", "-f", "tests/fixtures/lspci_output.txt", "--output", "json"])
        .assert()
        .success()
        .stdout(pred::contains("\"source\":"));
}

// ── Error handling & exit codes ───────────────────────────────────────────────

#[test]
fn invalid_hex_exits_nonzero_with_message() {
    cmd()
        .args(["-i", "ZZZZZZZZZZZZZZZZ"])
        .assert()
        .failure()
        .stderr(pred::contains("not valid hex"));
}

#[test]
fn empty_stdin_exits_nonzero() {
    cmd()
        .write_stdin("")
        .assert()
        .failure();
}

#[test]
fn valid_tlp_exits_zero() {
    cmd()
        .args(["-i", CONF_READ])
        .assert()
        .success();
}

// ── 4DW address display ───────────────────────────────────────────────────────

// 4DW MemReadReq: Fmt=001 (4DW no data), Type=00000, Length=1
// Req ID=0x0100, Tag=0x0A, upper addr=0xDEADBEEF, lower addr=0x12345670
const MEM_READ_4DW: &str = "20000001 01000AFF DEADBEEF 12345670";

#[test]
fn mem_read_4dw_shows_split_address() {
    cmd()
        .args(["-i", MEM_READ_4DW])
        .assert()
        .success()
        .stdout(pred::contains("MemReadReq"))
        .stdout(pred::contains("4DW no Data Header"))
        .stdout(pred::contains("Addr High (DW2)"))
        .stdout(pred::contains("Addr Low  (DW3)"))
        .stdout(pred::contains("0xDEADBEEF"))
        .stdout(pred::contains("0x12345670"));
}

#[test]
fn mem_read_3dw_shows_single_address() {
    // 3DW MemReadReq: Fmt=000 (3DW no data), Type=00000, Length=1
    // Req ID=0x0100, Tag=0x0A, addr=0xABCD1234
    cmd()
        .args(["-i", "00000001 01000AFF ABCD1234"])
        .assert()
        .success()
        .stdout(pred::contains("MemReadReq"))
        .stdout(pred::contains("Address (32b)"))
        .stdout(pred::contains("0xABCD1234"))
        .stdout(pred::contains("Addr High (DW2)").not());
}

// ── 0x-prefixed hex input ────────────────────────────────────────────────────

#[test]
fn hex_with_0x_prefix_is_accepted() {
    cmd()
        .args(["-i", "0x04000001 0x0000220f 0x01070000 0x9eece789"])
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"));
}

#[test]
fn hex_mixed_prefix_and_bare_is_accepted() {
    cmd()
        .args(["-i", "0x04000001 0000220f 0x01070000 9eece789"])
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"));
}

// ── Man page ──────────────────────────────────────────────────────────────────

#[test]
fn man_page_prints_and_exits_zero() {
    cmd()
        .args(["--man"])
        .assert()
        .success()
        .stdout(pred::contains(".TH rtlp_tool"))
        .stdout(pred::contains(".SH OPTIONS"))
        .stdout(pred::contains("TLP"));
}

// ── Shell completions ─────────────────────────────────────────────────────────

#[test]
fn completions_bash_prints_and_exits_zero() {
    cmd()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(pred::contains("rtlp-tool"));
}

// ── Edge cases & regression tests ────────────────────────────────────────────

/// An odd number of hex nibbles (e.g. a truncated DWord in a log) must produce
/// a clean error message — NOT a panic / index-out-of-bounds crash.
#[test]
fn odd_length_hex_exits_nonzero_not_panic() {
    cmd()
        // "04000001 0000220" — last DWord is only 7 chars (odd nibble count)
        .args(["-i", "04000001 0000220"])
        .assert()
        .failure()
        .stderr(pred::contains("not valid hex"));
}

/// --aer and --lspci are mutually exclusive; using both must produce a clear
/// error rather than silently ignoring --aer.
#[test]
fn aer_and_lspci_together_exits_nonzero() {
    cmd()
        .args(["--aer", "--lspci", "-f", "tests/fixtures/lspci_output.txt"])
        .assert()
        .failure()
        .stderr(pred::contains("mutually exclusive"));
}

/// --count 0 is valid; it processes zero TLPs and exits successfully with no
/// output (apart from the CSV header if --output csv is used).
#[test]
fn count_zero_produces_no_tlp_output() {
    cmd()
        .args(["-i", CONF_READ, "-i", CPL_DATA, "--count", "0"])
        .assert()
        .success()
        // No TLP type names should appear
        .stdout(pred::contains("ConfType0ReadReq").not())
        .stdout(pred::contains("CplData").not());
}
