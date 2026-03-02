use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::IsTerminal;
use std::io::Write;
use std::result::Result;

use rtlp_lib::{TlpPacket, TlpPacketHeader, TlpType, TlpFmt};
use rtlp_lib::{new_mem_req, new_conf_req, new_cmpl_req, new_msg_req};

use clap::{ArgEnum, CommandFactory, Parser};
use clap_complete::Shell;
use colored::Colorize;

#[macro_use] extern crate prettytable;
use prettytable::{Table, Row, Cell};
use prettytable::format;

// ── Output format ─────────────────────────────────────────────────────────────

#[derive(ArgEnum, Clone, Debug, PartialEq)]
enum OutputFormat {
    Table,
    Json,
    Csv,
}

// ── CLI args ──────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// TLP hex string(s) to parse. May be specified multiple times.
    /// Reads one TLP per line from stdin when omitted.
    #[clap(short, long, multiple_occurrences = true)]
    input: Vec<String>,

    /// Read TLP hex strings from a file (one per line)
    #[clap(short, long, value_name = "FILE")]
    file: Option<String>,

    /// Scan input for AER TLP headers (matches both 'TLP Header:' and 'HeaderLog:' patterns)
    #[clap(long)]
    aer: bool,

    /// Parse lspci -vvv output: extract non-zero HeaderLog entries and annotate
    /// each TLP with the PCIe device it belongs to
    #[clap(long)]
    lspci: bool,

    /// Process only the first N inputs (default: all)
    #[clap(short, long)]
    count: Option<usize>,

    /// Output format: table (default), json (ndjson), csv
    #[clap(long, arg_enum, default_value = "table", value_name = "FORMAT")]
    output: OutputFormat,

    /// Print shell completion script and exit
    #[clap(long, value_name = "SHELL")]
    completions: Option<Shell>,

    /// Print man page in troff format and exit
    #[clap(long)]
    man: bool,
}

// ── Collected TLP data (for all rendering modes) ──────────────────────────────

struct TlpData {
    index: usize,
    source: Option<String>,
    tlp_type: String,
    tlp_format: String,
    /// (field_name, offset_bits, length_bits, value)
    header_fields: Vec<(&'static str, &'static str, &'static str, String)>,
    /// (key, value)
    body_fields: Vec<(String, String)>,
}

// ── Config ────────────────────────────────────────────────────────────────────

struct Config {
    /// (raw bytes, optional source label)
    inputs: Vec<(Vec<u8>, Option<String>)>,
    count: Option<usize>,
    output: OutputFormat,
}

// ── AER / lspci scanner ───────────────────────────────────────────────────────

fn extract_tlp_from_line(line: &str) -> Option<String> {
    for pattern in &["TLP Header:", "HeaderLog:"] {
        if let Some(pos) = line.find(pattern) {
            let rest = line[pos + pattern.len()..].trim();
            let groups: Vec<&str> = rest.split_whitespace().take(4).collect();
            if !groups.is_empty() {
                return Some(groups.join(" "));
            }
        }
    }
    None
}

fn extract_pci_device(line: &str) -> Option<String> {
    // lspci lines: "0000:01:00.0 Non-Volatile memory controller: ..."
    let trimmed = line.trim();
    let (addr, rest) = match trimmed.split_once(' ') {
        Some(pair) => pair,
        None => return None,
    };
    // addr must contain ':' and '.' and only hex/colon/dot chars
    if !addr.contains(':') || !addr.contains('.') {
        return None;
    }
    let ok = addr.chars().all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.');
    if !ok {
        return None;
    }
    let label: String = rest.chars().take(50).collect();
    Some(format!("{} {}", addr, label))
}

fn scan_aer_lines(lines: &[String]) -> Vec<(String, Option<String>)> {
    let mut results = Vec::new();
    let mut current_device: Option<String> = None;
    for line in lines {
        if let Some(dev) = extract_pci_device(line) {
            current_device = Some(dev);
        }
        if let Some(tlp_hex) = extract_tlp_from_line(line) {
            results.push((tlp_hex, current_device.clone()));
        }
    }
    results
}

// ── Input helpers ─────────────────────────────────────────────────────────────

fn is_zero_header(hex: &str) -> bool {
    hex.chars()
        .filter(|c| !c.is_whitespace())
        .all(|c| c == '0')
}

/// Parse `lspci -vvv` output: extract HeaderLog entries that are non-zero,
/// and tag each one with the device address + name that owns it.
fn scan_lspci_lines(lines: &[String]) -> Vec<(String, Option<String>)> {
    let mut results = Vec::new();
    let mut current_device: Option<String> = None;
    for line in lines {
        if let Some(dev) = extract_pci_device(line) {
            current_device = Some(dev);
        }
        // Only match "HeaderLog:" (lspci specific), not "TLP Header:"
        if let Some(pos) = line.find("HeaderLog:") {
            let rest = line[pos + "HeaderLog:".len()..].trim();
            let groups: Vec<&str> = rest.split_whitespace().take(4).collect();
            if groups.is_empty() {
                continue;
            }
            let hex = groups.join(" ");
            if is_zero_header(&hex) {
                continue; // no TLP captured for this device
            }
            results.push((hex, current_device.clone()));
        }
    }
    results
}

fn read_lines_from<R: BufRead>(reader: R) -> Vec<String> {
    reader
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

// ── Color helpers ─────────────────────────────────────────────────────────────

/// Returns true when color should be applied: stdout is a tty and NO_COLOR is unset.
fn use_color() -> bool {
    std::io::stdout().is_terminal() && std::env::var("NO_COLOR").is_err()
}

/// prettytable style_spec string for the TLP type cell.
fn tlp_type_style(tlp_type: &str) -> &'static str {
    if tlp_type.starts_with("Mem") || tlp_type.starts_with("IO") || tlp_type.contains("Atomic") {
        "Fb" // blue — memory / IO / atomic
    } else if tlp_type.starts_with("Conf") {
        "Fc" // cyan — configuration
    } else if tlp_type.starts_with("Msg") {
        "Fm" // magenta — message
    } else if tlp_type.starts_with("Cpl") {
        "Fg" // green — completion
    } else if tlp_type.starts_with("Error") {
        "Fr" // red — parse error
    } else {
        ""
    }
}

/// prettytable style_spec for common-header field values.
fn header_val_style(name: &str, value: &str) -> &'static str {
    match name {
        "Ep" if value != "0" => "bFr", // bold red  — Error Poison bit set
        "TC" if value != "0" => "Fy",  // yellow    — non-default traffic class
        "TD" if value != "0" => "Fy",  // yellow    — ECRC digest present
        "TH" if value != "0" => "Fc",  // cyan      — TLP processing hints active
        "AT" if value != "0" => "Fc",  // cyan      — address translation active
        _ => "",
    }
}

/// prettytable style_spec for body field values.
fn body_val_style(key: &str, value: &str) -> &'static str {
    match key {
        "Compl Status" if value != "0x0" => "bFr", // bold red — non-OK completion
        _ => "",
    }
}

// ── Config parsing ────────────────────────────────────────────────────────────

enum ParseConfigError {
    InvalidInput(usize),
}

impl Config {
    fn remove_whitespace(s: &str) -> String {
        // Strip optional 0x/0X prefix from each whitespace-separated token,
        // then concatenate — allows inputs like "0x04000001 0x0000220f ..."
        s.split_whitespace()
            .map(|tok| tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")).unwrap_or(tok))
            .collect()
    }

    fn convert_to_vec(s: &str) -> Result<Vec<u8>, ()> {
        const RADIX: u32 = 16;
        let mut nibbles: Vec<u8> = Vec::new();
        for c in s.chars() {
            match c.to_digit(RADIX) {
                Some(d) => nibbles.push(d as u8),
                None => return Err(()),
            }
        }
        let mut result: Vec<u8> = Vec::new();
        for chunk in nibbles.chunks(2) {
            result.push((chunk[0] << 4) + chunk[1]);
        }
        Ok(result)
    }

    fn new(
        raw_inputs: Vec<(String, Option<String>)>,
        count: Option<usize>,
        output: OutputFormat,
    ) -> Result<Config, ParseConfigError> {
        let mut inputs = Vec::new();
        for (i, (raw, source)) in raw_inputs.into_iter().enumerate() {
            let cleaned = Config::remove_whitespace(&raw);
            match Config::convert_to_vec(&cleaned) {
                Ok(bytes) => inputs.push((bytes, source)),
                Err(()) => return Err(ParseConfigError::InvalidInput(i + 1)),
            }
        }
        Ok(Config { inputs, count, output })
    }
}

// ── TlpTool ───────────────────────────────────────────────────────────────────

struct TlpTool {
    config: Config,
}

impl TlpTool {
    fn new(cfg: Config) -> TlpTool {
        TlpTool { config: cfg }
    }

    // ── collect methods ────────────────────────────────────────────────────────

    fn collect_header_fields(
        hdr: &TlpPacketHeader,
    ) -> Vec<(&'static str, &'static str, &'static str, String)> {
        vec![
            ("Fmt",     "0",  "3",  format!("{}", hdr.get_format())),
            ("Type",    "3",  "5",  format!("{}", hdr.get_type())),
            ("T9",      "8",  "1",  format!("{}", hdr.get_t9())),
            ("TC",      "9",  "3",  format!("{}", hdr.get_tc())),
            ("T8",      "12", "1",  format!("{}", hdr.get_t8())),
            ("Attr_b2", "13", "1",  format!("{}", hdr.get_attr_b2())),
            ("LN",      "14", "1",  format!("{}", hdr.get_ln())),
            ("TH",      "15", "1",  format!("{}", hdr.get_th())),
            ("Td",      "16", "1",  format!("{}", hdr.get_td())),
            ("Ep",      "17", "1",  format!("{}", hdr.get_ep())),
            ("Attr",    "18", "2",  format!("{}", hdr.get_attr())),
            ("AT",      "20", "2",  format!("{}", hdr.get_at())),
            ("Length",  "22", "10", format!("{}", hdr.get_length())),
        ]
    }

    fn collect_mem_req(tlp: &TlpPacket) -> Vec<(String, String)> {
        let tlpf = tlp.get_tlp_format();
        let mr = new_mem_req(tlp.get_data(), &tlpf);
        let is_4dw = matches!(tlpf, TlpFmt::NoDataHeader4DW | TlpFmt::WithDataHeader4DW);
        let addr = mr.address();
        let mut fields = vec![
            ("Req ID".into(),      format!("{:#X}", mr.req_id())),
            ("Tag".into(),         format!("{:#X}", mr.tag())),
            ("Last DW BE".into(),  format!("{:#X}", mr.ldwbe())),
            ("First DW BE".into(), format!("{:#X}", mr.fdwbe())),
        ];
        if is_4dw {
            fields.push(("Addr High (DW2)".into(), format!("{:#010X}", (addr >> 32) as u32)));
            fields.push(("Addr Low  (DW3)".into(), format!("{:#010X}", (addr & 0xFFFF_FFFF) as u32)));
        } else {
            fields.push(("Address (32b)".into(), format!("{:#010X}", addr as u32)));
        }
        fields
    }

    fn collect_cfg_req(tlp: &TlpPacket) -> Vec<(String, String)> {
        let tlpf = tlp.get_tlp_format();
        if tlpf == TlpFmt::NoDataHeader4DW || tlpf == TlpFmt::WithDataHeader4DW {
            return vec![("Error".into(), "Configuration Requests are always 3DW".into())];
        }
        let cfg = new_conf_req(tlp.get_data(), &tlpf);
        vec![
            ("Req ID".into(),     format!("{:#X}", cfg.req_id())),
            ("Tag".into(),        format!("{:#X}", cfg.tag())),
            ("Bus".into(),        format!("{:#X}", cfg.bus_nr())),
            ("Device".into(),     format!("{:#X}", cfg.dev_nr())),
            ("Function".into(),   format!("{:#X}", cfg.func_nr())),
            ("Ext Reg Nr".into(), format!("{:#X}", cfg.ext_reg_nr())),
            ("Reg Nr".into(),     format!("{:#X}", cfg.reg_nr())),
        ]
    }

    fn collect_cmpl(tlp: &TlpPacket) -> Vec<(String, String)> {
        let tlpf = tlp.get_tlp_format();
        if tlpf == TlpFmt::NoDataHeader4DW || tlpf == TlpFmt::WithDataHeader4DW {
            return vec![("Error".into(), "Completions are always 3DW".into())];
        }
        let cpl = new_cmpl_req(tlp.get_data(), &tlpf);
        vec![
            ("Compl ID".into(),                    format!("{:#X}", cpl.cmpl_id())),
            ("Compl Status".into(),                format!("{:#X}", cpl.cmpl_stat())),
            ("Byte Count Modified (PCI-X)".into(), format!("{:#X}", cpl.bcm())),
            ("Byte Count".into(),                  format!("{:#X}", cpl.byte_cnt())),
            ("Req ID".into(),                      format!("{:#X}", cpl.req_id())),
            ("Tag".into(),                         format!("{:#X}", cpl.tag())),
            ("Lower Address".into(),               format!("{:#X}", cpl.laddr())),
        ]
    }

    fn collect_msg(tlp: &TlpPacket) -> Vec<(String, String)> {
        let tlpf = tlp.get_tlp_format();
        let msg = new_msg_req(tlp.get_data(), &tlpf);
        vec![
            ("Req ID".into(),       format!("{:#X}", msg.req_id())),
            ("Tag".into(),          format!("{:#X}", msg.tag())),
            ("Message Code".into(), format!("{:#X}", msg.msg_code())),
            ("Message DW3".into(),  format!("{:#X}", msg.dw3())),
            ("Message DW4".into(),  format!("{:#X}", msg.dw4())),
        ]
    }

    fn collect_body_fields(tlp: &TlpPacket) -> Vec<(String, String)> {
        match tlp.get_tlp_type() {
            Ok(tlpt) => match tlpt {
                TlpType::MemReadReq
                | TlpType::MemReadLockReq
                | TlpType::MemWriteReq
                | TlpType::IOReadReq
                | TlpType::IOWriteReq
                | TlpType::FetchAddAtomicOpReq
                | TlpType::SwapAtomicOpReq
                | TlpType::CompareSwapAtomicOpReq => Self::collect_mem_req(tlp),

                TlpType::ConfType0ReadReq
                | TlpType::ConfType0WriteReq
                | TlpType::ConfType1ReadReq
                | TlpType::ConfType1WriteReq => Self::collect_cfg_req(tlp),

                TlpType::MsgReq | TlpType::MsgReqData => Self::collect_msg(tlp),

                TlpType::Cpl
                | TlpType::CplData
                | TlpType::CplLocked
                | TlpType::CplDataLocked => Self::collect_cmpl(tlp),

                TlpType::LocalTlpPrefix | TlpType::EndToEndTlpPrefix => {
                    vec![("Note".into(), "Display not implemented for TLP prefix types".into())]
                }
            },
            Err(e) => vec![("Error".into(), format!("Cannot parse TLP type: {:?}", e))],
        }
    }

    fn collect_tlp(index: usize, tlp: &TlpPacket, source: Option<String>) -> TlpData {
        let tlp_type = match tlp.get_tlp_type() {
            Ok(t)  => format!("{:?}", t),
            Err(e) => format!("Error: {:?}", e),
        };
        TlpData {
            index,
            source,
            tlp_type,
            tlp_format: format!("{}", tlp.get_tlp_format()),
            header_fields: Self::collect_header_fields(tlp.get_header()),
            body_fields: Self::collect_body_fields(tlp),
        }
    }

    // ── render methods ─────────────────────────────────────────────────────────

    fn render_table(data: &TlpData) {
        let color = use_color();

        // Type / source banner
        let mut t = Table::new();
        let type_style = if color { tlp_type_style(&data.tlp_type) } else { "" };
        t.add_row(Row::new(vec![
            Cell::new("TLP Type"),
            Cell::new(&data.tlp_type).style_spec(type_style),
            Cell::new(&data.tlp_format),
        ]));
        if let Some(src) = &data.source {
            t.add_row(Row::new(vec![
                Cell::new("Source").style_spec(if color { "b" } else { "" }),
                Cell::new(src).style_spec(if color { "Fc" } else { "" }),
                Cell::new(""),
            ]));
        }
        t.printstd();

        // Common header fields
        let name_style = if color { "b" } else { "" };
        let mut t = Table::new();
        t.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        t.set_titles(row!["Field Name", "Offset\n(bits)", "Length\n(bits)", "Value"]);
        for (name, offset, length, value) in &data.header_fields {
            let val_style = if color { header_val_style(name, value) } else { "" };
            t.add_row(Row::new(vec![
                Cell::new(name).style_spec(name_style),
                Cell::new(offset),
                Cell::new(length),
                Cell::new(value).style_spec(val_style),
            ]));
        }
        t.printstd();

        // Body fields
        let mut t = Table::new();
        t.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        t.set_titles(row!["TLP:", &data.tlp_format]);
        for (k, v) in &data.body_fields {
            let val_style = if color { body_val_style(k, v) } else { "" };
            t.add_row(Row::new(vec![
                Cell::new(k).style_spec(name_style),
                Cell::new(v).style_spec(val_style),
            ]));
        }
        t.printstd();
    }

    fn render_json(data: &TlpData) {
        let mut parts = Vec::new();
        parts.push(format!("\"index\":{}", data.index));
        if let Some(src) = &data.source {
            parts.push(format!("\"source\":\"{}\"", src.replace('"', "\\\"")));
        }
        parts.push(format!("\"tlp_type\":\"{}\"", data.tlp_type));
        parts.push(format!("\"tlp_format\":\"{}\"", data.tlp_format));

        let hdr: Vec<String> = data.header_fields.iter()
            .map(|(name, _, _, val)| format!("\"{}\":\"{}\"", name, val))
            .collect();
        parts.push(format!("\"header\":{{{}}}", hdr.join(",")));

        let body: Vec<String> = data.body_fields.iter()
            .map(|(k, v)| {
                let ek = k.replace('"', "\\\"");
                let ev = v.replace('"', "\\\"");
                format!("\"{}\":\"{}\"", ek, ev)
            })
            .collect();
        parts.push(format!("\"body\":{{{}}}", body.join(",")));

        println!("{{{}}}", parts.join(","));
    }

    fn render_csv_header() {
        println!("index,source,tlp_type,tlp_format,section,key,value");
    }

    fn render_csv(data: &TlpData) {
        let src = data.source.as_deref().unwrap_or("").replace(',', ";");
        let idx  = data.index;
        let tt   = data.tlp_type.replace(',', ";");
        let tf   = data.tlp_format.replace(',', ";");
        for (name, _, _, val) in &data.header_fields {
            println!("{},{},{},{},header,{},{}", idx, src, tt, tf, name, val.replace(',', ";"));
        }
        for (k, v) in &data.body_fields {
            println!(
                "{},{},{},{},body,{},{}",
                idx, src, tt, tf,
                k.replace(',', ";"),
                v.replace(',', ";")
            );
        }
    }

    // ── run ────────────────────────────────────────────────────────────────────

    fn run(&self) -> i32 {
        // Configure the `colored` crate once — matches prettytable's own tty/NO_COLOR check.
        colored::control::set_override(use_color());

        let limit = self.config.count.unwrap_or(self.config.inputs.len());
        let multiple = limit > 1 && self.config.output == OutputFormat::Table;
        let mut had_error = false;

        if self.config.output == OutputFormat::Csv {
            Self::render_csv_header();
        }

        for (i, (bytes, source)) in self.config.inputs.iter().take(limit).enumerate() {
            let tlp = TlpPacket::new(bytes.clone());
            if tlp.get_tlp_type().is_err() {
                had_error = true;
            }
            let data = Self::collect_tlp(i + 1, &tlp, source.clone());

            if multiple {
                println!("\n{}", format!("=== TLP #{} ===", i + 1).bold().yellow());
            }
            match self.config.output {
                OutputFormat::Table => Self::render_table(&data),
                OutputFormat::Json  => Self::render_json(&data),
                OutputFormat::Csv   => Self::render_csv(&data),
            }
        }

        if had_error { 1 } else { 0 }
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();

    // Shell completions: print and exit
    if let Some(shell) = args.completions {
        let mut cmd = Args::command();
        clap_complete::generate(shell, &mut cmd, "rtlp-tool", &mut std::io::stdout());
        return;
    }

    // Man page: render troff to stdout and exit
    if args.man {
        let cmd = Args::command();
        let man = clap_mangen::Man::new(cmd);
        let mut buf = Vec::new();
        if let Err(e) = man.render(&mut buf) {
            eprintln!("error: failed to generate man page: {}", e);
            std::process::exit(1);
        }
        std::io::stdout().write_all(&buf).unwrap();
        return;
    }

    // Helper: read raw text lines from -i flags / -f file / stdin
    let read_text_lines = |input: &Vec<String>, file: &Option<String>| -> Vec<String> {
        if !input.is_empty() {
            input.clone()
        } else if let Some(path) = file {
            match File::open(path) {
                Ok(f) => read_lines_from(BufReader::new(f)),
                Err(e) => {
                    eprintln!("error: cannot open '{}': {}", path, e);
                    std::process::exit(1);
                }
            }
        } else {
            read_lines_from(std::io::stdin().lock())
        }
    };

    // Collect (hex_string, source_label) pairs according to mode
    let raw_inputs: Vec<(String, Option<String>)> = if args.lspci {
        // lspci -vvv mode: extract non-zero HeaderLog entries with device context
        let lines = read_text_lines(&args.input, &args.file);
        let found = scan_lspci_lines(&lines);
        if found.is_empty() {
            eprintln!(
                "error: no non-zero HeaderLog entries found in input\n\
                 hint: run 'lspci -vvv | rtlp-tool --lspci' or \
                 'rtlp-tool --lspci -f <lspci_output.txt>'"
            );
            std::process::exit(1);
        }
        found

    } else if args.aer {
        // AER mode: extract TLP Header: and HeaderLog: patterns
        let lines = read_text_lines(&args.input, &args.file);
        let found = scan_aer_lines(&lines);
        if found.is_empty() {
            eprintln!(
                "error: no TLP headers found in input \
                 (looked for 'TLP Header:' and 'HeaderLog:')"
            );
            std::process::exit(1);
        }
        found

    } else if let Some(ref path) = args.file {
        // File mode: one hex string per line
        match File::open(path) {
            Ok(f) => read_lines_from(BufReader::new(f))
                .into_iter()
                .map(|l| (l, None))
                .collect(),
            Err(e) => {
                eprintln!("error: cannot open '{}': {}", path, e);
                std::process::exit(1);
            }
        }

    } else if !args.input.is_empty() {
        // Direct -i flags
        args.input.into_iter().map(|s| (s, None)).collect()

    } else {
        // Stdin fallback
        read_lines_from(std::io::stdin().lock())
            .into_iter()
            .map(|l| (l, None))
            .collect()
    };

    if raw_inputs.is_empty() {
        eprintln!(
            "error: no input provided — use -i <HEX>, -f <FILE>, --lspci, --aer, or pipe via stdin"
        );
        std::process::exit(1);
    }

    match Config::new(raw_inputs, args.count, args.output) {
        Ok(c) => std::process::exit(TlpTool::new(c).run()),
        Err(ParseConfigError::InvalidInput(n)) => {
            eprintln!("input #{n} is not valid hex");
            std::process::exit(1);
        }
    }
}
