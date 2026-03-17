use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str as pred;

// Two TLP hex strings used across multiple tests
const CONF_READ: &str = "04000001 0000220f 01070000 9eece789";
const CPL_DATA: &str = "4a000001 2001FF00 C281FF10 00000000";

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
    assert_eq!(
        lines.len(),
        2,
        "expected exactly 2 JSON lines (ndjson), got:\n{}",
        text
    );
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
        .args([
            "--lspci",
            "-f",
            "tests/fixtures/lspci_output.txt",
            "--output",
            "json",
        ])
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
    cmd().write_stdin("").assert().failure();
}

#[test]
fn valid_tlp_exits_zero() {
    cmd().args(["-i", CONF_READ]).assert().success();
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

// ── Flit mode (PCIe 6.0) ──────────────────────────────────────────────────────
//
// Flit-mode DW0 byte 0 is a flat 8-bit type code (completely different from the
// non-flit Fmt[2:0] | Type[4:0] split).
//
//   0x00 = NOP            — 1 DW base header, no payload
//   0x03 = MemRead32      — 3 DW base header, no data payload
//   0x40 = MemWrite32     — 3 DW base header + data payload
//
// These byte patterns are taken directly from the rtlp-lib 0.5.0 FlitTlpType
// TryFrom<u8> table and the PCIe 6.0 spec.

// NOP flit:  type=0x00, OHC=0x00, length=0x0000  (1 DW = 4 bytes)
const FLIT_NOP: &str = "00 00 00 00";

// MemRead32 flit: type=0x03, OHC=0x00, length=1 DW, then DW1+DW2 (3 DWs = 12 bytes)
const FLIT_MEM_READ32: &str = "03 00 00 01 01 00 0A FF AB CD 12 34";

#[test]
fn flit_nop_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_NOP])
        .assert()
        .success()
        .stdout(pred::contains("NOP"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_shows_flit_header_fields() {
    cmd()
        .args(["--flit", "-i", FLIT_NOP])
        .assert()
        .success()
        // DW0 field names specific to flit mode
        .stdout(pred::contains("Type Code"))
        .stdout(pred::contains("OHC"))
        .stdout(pred::contains("Length"));
}

#[test]
fn flit_mem_read32_shows_correct_type() {
    cmd()
        .args(["--flit", "-i", FLIT_MEM_READ32])
        .assert()
        .success()
        .stdout(pred::contains("Memory Read (32-bit)"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_shows_raw_dw_body() {
    cmd()
        .args(["--flit", "-i", FLIT_MEM_READ32])
        .assert()
        .success()
        // Body shows raw DW words (DW0 = first 4 bytes of packet)
        .stdout(pred::contains("DW0"))
        .stdout(pred::contains("DW1"))
        .stdout(pred::contains("DW2"));
}

#[test]
fn flit_json_output_has_flit_mode_true() {
    cmd()
        .args(["--flit", "-i", FLIT_NOP, "--output", "json"])
        .assert()
        .success()
        .stdout(pred::contains("\"flit_mode\":true"))
        .stdout(pred::contains("\"tlp_type\":\"NOP\""))
        .stdout(pred::contains("\"tlp_format\":\"Flit Mode (PCIe 6.0)\""));
}

#[test]
fn non_flit_json_has_flit_mode_false() {
    cmd()
        .args(["-i", CONF_READ, "--output", "json"])
        .assert()
        .success()
        .stdout(pred::contains("\"flit_mode\":false"));
}

#[test]
fn flit_csv_output_shows_flit_format_and_fields() {
    cmd()
        .args(["--flit", "-i", FLIT_NOP, "--output", "csv"])
        .assert()
        .success()
        // tlp_format column carries "Flit Mode (PCIe 6.0)" — backward-compatible header
        .stdout(pred::starts_with(
            "index,source,tlp_type,tlp_format,section,key,value\n",
        ))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"))
        .stdout(pred::contains(",header,Type Code,"));
}

#[test]
fn flit_multiple_tlps_shows_separators() {
    cmd()
        .args(["--flit", "-i", FLIT_NOP, "-i", FLIT_MEM_READ32])
        .assert()
        .success()
        .stdout(pred::contains("=== TLP #1 ==="))
        .stdout(pred::contains("=== TLP #2 ==="))
        .stdout(pred::contains("NOP"))
        .stdout(pred::contains("Memory Read (32-bit)"));
}

#[test]
fn flit_file_input_two_tlps() {
    cmd()
        .args(["--flit", "-f", "tests/fixtures/flit_tlps.txt"])
        .assert()
        .success()
        .stdout(pred::contains("=== TLP #1 ==="))
        .stdout(pred::contains("=== TLP #2 ==="))
        .stdout(pred::contains("NOP"))
        .stdout(pred::contains("Memory Read (32-bit)"));
}

#[test]
fn flit_stdin_input() {
    cmd()
        .args(["--flit"])
        .write_stdin(format!("{}\n", FLIT_NOP))
        .assert()
        .success()
        .stdout(pred::contains("NOP"));
}

#[test]
fn flit_invalid_type_code_exits_nonzero() {
    // 0xFF is not a defined FlitTlpType code in rtlp-lib 0.5.0.
    // TlpPacket::new(..., TlpMode::Flit) should return Err → exit code 1.
    cmd()
        .args(["--flit", "-i", "FF 00 00 00"])
        .assert()
        .failure()
        .stderr(pred::contains("cannot be parsed"));
}

#[test]
fn flit_count_limits_output() {
    cmd()
        .args([
            "--flit",
            "-i",
            FLIT_NOP,
            "-i",
            FLIT_MEM_READ32,
            "--count",
            "1",
        ])
        .assert()
        .success()
        .stdout(pred::contains("NOP"))
        .stdout(pred::contains("Memory Read (32-bit)").not());
}

// ── All supported flit type codes (rtlp-lib 0.5.0) ──────────────────────────
//
// One test per type ensures full coverage of the flit type table documented in
// the README.  Byte patterns were validated against the live binary.
//
// DW0 layout:  byte0=TypeCode  byte1=OHC  bytes2-3=Length(DWs)
//
// Types 0x42 (I/O Write) and 0x44 (Config Type 0 Write) carry a mandatory OHC
// extension (bit 0 of OHC byte must be set); both positive and negative tests
// are included for those types.

// 0x22 — UIO Memory Read (64-bit): 4 DW base header
const FLIT_UIO_MEM_READ64: &str = "22 00 00 01 01 00 0A FF DE AD BE EF AB CD 12 34";

// 0x30 — Message routed to RC: 4 DW base header, length=0
const FLIT_MSG_ROUTED_RC: &str = "30 00 00 00 00 00 0A 00 00 00 00 00 00 00 00 00";

// 0x40 — Memory Write (32-bit): 3 DW header + 1 DW data
const FLIT_MEM_WRITE32: &str = "40 00 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF";

// 0x42 — I/O Write: mandatory OHC (byte 1 = 0x01) + 3 DW header + 1 DW data + 1 OHC DW
const FLIT_IO_WRITE: &str = "42 01 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF 00 00 00 00";

// 0x44 — Config Type 0 Write: mandatory OHC (byte 1 = 0x01)
const FLIT_CFG_TYPE0_WRITE: &str =
    "44 01 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF 00 00 00 00";

// 0x4C — FetchAdd AtomicOp (32-bit): 3 DW header + 1 DW operand
const FLIT_FETCHADD32: &str = "4C 00 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF";

// 0x4E — CompareSwap AtomicOp (32-bit): 3 DW header + 2 DW operands
const FLIT_COMPARESWAP32: &str =
    "4E 00 00 02 01 00 0A FF AB CD 12 34 DE AD BE EF 00 11 22 33";

// 0x5B — Deferrable Memory Write (32-bit): 3 DW header + 1 DW data
const FLIT_DEFERRABLE_MEM_WRITE32: &str = "5B 00 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF";

// 0x61 — UIO Memory Write (64-bit): 4 DW header + 1 DW data
const FLIT_UIO_MEM_WRITE64: &str =
    "61 00 00 01 01 00 0A FF DE AD BE EF AB CD 12 34 00 11 22 33";

// 0x70 — Message with Data routed to RC: 4 DW header + 1 DW data
const FLIT_MSG_DATA_ROUTED_RC: &str =
    "70 00 00 01 00 00 0A 00 00 00 00 00 00 00 00 00 DE AD BE EF";

// 0x8D — Local TLP Prefix: 2 DW
const FLIT_LOCAL_TLP_PREFIX: &str = "8D 00 00 00 00 00 00 00";

#[test]
fn flit_uio_mem_read64_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_UIO_MEM_READ64])
        .assert()
        .success()
        .stdout(pred::contains("UIO Memory Read (64-bit)"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_message_routed_to_rc_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_MSG_ROUTED_RC])
        .assert()
        .success()
        .stdout(pred::contains("Message routed to RC"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_mem_write32_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_MEM_WRITE32])
        .assert()
        .success()
        .stdout(pred::contains("Memory Write (32-bit)"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_io_write_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_IO_WRITE])
        .assert()
        .success()
        .stdout(pred::contains("I/O Write"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

/// I/O Write (0x42) requires mandatory OHC — OHC byte 0x00 must be rejected.
#[test]
fn flit_io_write_without_mandatory_ohc_fails() {
    cmd()
        .args([
            "--flit",
            "-i",
            "42 00 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF",
        ])
        .assert()
        .failure()
        .stderr(pred::contains("cannot be parsed"));
}

#[test]
fn flit_cfg_type0_write_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_CFG_TYPE0_WRITE])
        .assert()
        .success()
        .stdout(pred::contains("Config Type 0 Write"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

/// Config Type 0 Write (0x44) requires mandatory OHC — OHC byte 0x00 must be rejected.
#[test]
fn flit_cfg_type0_write_without_mandatory_ohc_fails() {
    cmd()
        .args([
            "--flit",
            "-i",
            "44 00 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF",
        ])
        .assert()
        .failure()
        .stderr(pred::contains("cannot be parsed"));
}

#[test]
fn flit_fetchadd32_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_FETCHADD32])
        .assert()
        .success()
        .stdout(pred::contains("FetchAdd AtomicOp (32-bit)"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_compareswap32_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_COMPARESWAP32])
        .assert()
        .success()
        .stdout(pred::contains("CompareSwap AtomicOp (32-bit)"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_deferrable_mem_write32_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_DEFERRABLE_MEM_WRITE32])
        .assert()
        .success()
        .stdout(pred::contains("Deferrable Memory Write (32-bit)"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_uio_mem_write64_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_UIO_MEM_WRITE64])
        .assert()
        .success()
        .stdout(pred::contains("UIO Memory Write (64-bit)"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_message_with_data_routed_to_rc_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_MSG_DATA_ROUTED_RC])
        .assert()
        .success()
        .stdout(pred::contains("Message with Data routed to RC"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

#[test]
fn flit_local_tlp_prefix_parses_correctly() {
    cmd()
        .args(["--flit", "-i", FLIT_LOCAL_TLP_PREFIX])
        .assert()
        .success()
        .stdout(pred::contains("Local TLP Prefix"))
        .stdout(pred::contains("Flit Mode (PCIe 6.0)"));
}

/// Without --flit, standard non-flit TLPs still parse correctly (backward-compat).
#[test]
fn without_flit_flag_non_flit_parsing_unchanged() {
    cmd()
        .args(["-i", CONF_READ])
        .assert()
        .success()
        .stdout(pred::contains("ConfType0ReadReq"))
        // Flit-specific strings must NOT appear in non-flit output
        .stdout(pred::contains("Flit Mode (PCIe 6.0)").not())
        .stdout(pred::contains("Type Code").not());
}

// ── Header field value correctness ───────────────────────────────────────────
//
// These tests verify the ACTUAL VALUES of DW0 bit-fields, not just that the
// field names are present.  This catches the class of bug where tlp.data()
// (bytes after DW0) is accidentally used for header extraction instead of the
// original raw bytes.
//
// CONF_READ = "04000001 0000220f 01070000 9eece789"
//   DW0 = 04 00 00 01
//   Fmt    = (0x04 >> 5) & 0x7 = 0  → "3DW no data"
//   Type   = 0x04 & 0x1F      = 4  → ConfType0Read
//   TC     = (0x00 >> 4) & 0x7 = 0
//   Attr   = (0x00 >> 4) & 0x3 = 0
//   AT     = (0x00 >> 2) & 0x3 = 0
//   Length = ((0x00 & 0x3) << 8) | 0x01 = 1

#[test]
fn header_fields_fmt_and_type_are_correct() {
    cmd()
        .args(["-i", CONF_READ, "--output", "json"])
        .assert()
        .success()
        // Fmt=0 (3DW no data), Type=4 (Config Type 0)
        .stdout(pred::contains("\"Fmt\":\"0\""))
        .stdout(pred::contains("\"Type\":\"4\""));
}

#[test]
fn header_fields_length_is_correct() {
    cmd()
        .args(["-i", CONF_READ, "--output", "json"])
        .assert()
        .success()
        // DW0[9:0] = 1 DW payload length
        .stdout(pred::contains("\"Length\":\"1\""));
}

#[test]
fn header_fields_tc_attr_at_are_zero_for_conf_read() {
    cmd()
        .args(["-i", CONF_READ, "--output", "json"])
        .assert()
        .success()
        .stdout(pred::contains("\"TC\":\"0\""))
        .stdout(pred::contains("\"Attr\":\"0\""))
        .stdout(pred::contains("\"AT\":\"0\""));
}

// CPL_DATA = "4a000001 2001FF00 C281FF10 00000000"
//   DW0 = 4a 00 00 01
//   Fmt    = (0x4a >> 5) & 0x7 = 0b010 = 2  → "3DW with data"
//   Type   = 0x4a & 0x1F      = 0x0a = 10  → Cpl with Data (CplData)
//   Length = ((0x00 & 0x3) << 8) | 0x01 = 1

#[test]
fn header_fields_cpl_data_fmt_and_length() {
    cmd()
        .args(["-i", CPL_DATA, "--output", "json"])
        .assert()
        .success()
        // Fmt=2 (3DW with data)
        .stdout(pred::contains("\"Fmt\":\"2\""))
        .stdout(pred::contains("\"Length\":\"1\""));
}
