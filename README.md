Rust TLP Tool 
=============

Simple Tool to parse PCI TLP headers into the human readable form.

## How I may know my TLP?
PCI TLP Headers are very usefull during troubleshooting driver or PCI Devices.
During Debugging we find TLP Headers for analysis in:
 * Devices Header Log Register 
 * Reported by AER

### Devices Header Log
Device Header Log can be find via lspci inside Capabilities Register.
Below I show HeaderLog of NVMe drive on my system:

```bash
lspci -s 01:00.0 -vv
01:00.0 Non-Volatile memory controller: Phison Electronics Corporation E16 PCIe4 NVMe Controller (rev 01) (prog-if 02 [NVM Express])
...
        Capabilities: [1e0 v1] Data Link Feature <?>
        Capabilities: [200 v2] Advanced Error Reporting
                UESta:  DLP- SDES- TLP- FCP- CmpltTO- CmpltAbrt- UnxCmplt- RxOF- MalfTLP- ECRC- UnsupReq- ACSViol-
                UEMsk:  DLP- SDES- TLP- FCP- CmpltTO- CmpltAbrt- UnxCmplt- RxOF- MalfTLP- ECRC- UnsupReq- ACSViol-
                UESvrt: DLP+ SDES- TLP- FCP+ CmpltTO- CmpltAbrt- UnxCmplt- RxOF- MalfTLP+ ECRC- UnsupReq- ACSViol-
                CESta:  RxErr- BadTLP- BadDLLP- Rollover- Timeout- AdvNonFatalErr-
                CEMsk:  RxErr- BadTLP- BadDLLP- Rollover- Timeout- AdvNonFatalErr+
                AERCap: First Error Pointer: 00, ECRCGenCap- ECRCGenEn- ECRCChkCap+ ECRCChkEn-
                        MultHdrRecCap- MultHdrRecEn- TLPPfxPres- HdrLogCap-
                HeaderLog: 04000001 0000220f 01070000 9eece789

```
This problematic TLP Header "04000001 0000220f 01070000 9eece789" can be easily parsed by rtlp-tool

```bash
rtlp-tool -i "04000001 0000220f 01070000 9eece789"
+----------+----------------------------+--------------------+
| TLP Type | Type 0 Config Read Request | 3DW no Data Header |
+----------+----------------------------+--------------------+
+------------+--------+--------+-------+
| Field Name | Offset | Length | Value |
|            | (bits) | (bits) |       |
+------------+--------+--------+-------+
| Fmt        | 0      | 3      | 0     |
| Type       | 3      | 5      | 4     |
| T9         | 8      | 1      | 0     |
| TC         | 9      | 3      | 0     |
| T8         | 12     | 1      | 0     |
| Attr_b2    | 13     | 1      | 0     |
| LN         | 14     | 1      | 0     |
| TH         | 15     | 1      | 0     |
| Td         | 16     | 1      | 0     |
| Ep         | 17     | 1      | 0     |
| Attr       | 18     | 2      | 0     |
| AT         | 20     | 2      | 0     |
| Length     | 22     | 10     | 1     |
+------------+--------+--------+-------+
+------------+--------------------+
| TLP:       | 3DW no Data Header |
+------------+--------------------+
| Req ID     | 0x0                |
| Tag        | 0x22               |
| Bus        | 0x1                |
| Device     | 0x0                |
| Function   | 0x7                |
| Ext Reg Nr | 0x0                |
| Reg Nr     | 0x0                |
+------------+--------------------+

```
### AER Report
AER reports usually happens when error is reported by PCIe device (that require AER capability to be enabled by OS or/and System Firmware such as BIOS/UEFI)
Below I show example from Linux kernel documentation `Documentation/PCI/pcieaer-howto.txt`

```bash
0000:40:00.0: PCIe Bus Error: severity=Uncorrected (Fatal), type=Transaction Layer, id=0500(Requester ID)
0000:40:00.0:   device [8086:0329] error status/mask=00100000/00000000
0000:40:00.0:    [20] Unsupported Request    (First)
0000:40:00.0:   TLP Header: 04000001 00200a03 05010000 00050100
```

TLP Header `04000001 00200a03 05010000 00050100` can be parsed by rtlp-tool via:

```bash
rtlp-tool -i "04000001 00200a03 05010000 00050100"
+----------+----------------------------+--------------------+
| TLP Type | Type 0 Config Read Request | 3DW no Data Header |
+----------+----------------------------+--------------------+
+------------+--------+--------+-------+
| Field Name | Offset | Length | Value |
|            | (bits) | (bits) |       |
+------------+--------+--------+-------+
| Fmt        | 0      | 3      | 0     |
| Type       | 3      | 5      | 4     |
| T9         | 8      | 1      | 0     |
| TC         | 9      | 3      | 0     |
| T8         | 12     | 1      | 0     |
| Attr_b2    | 13     | 1      | 0     |
| LN         | 14     | 1      | 0     |
| TH         | 15     | 1      | 0     |
| Td         | 16     | 1      | 0     |
| Ep         | 17     | 1      | 0     |
| Attr       | 18     | 2      | 0     |
| AT         | 20     | 2      | 0     |
| Length     | 22     | 10     | 1     |
+------------+--------+--------+-------+
+------------+--------------------+
| TLP:       | 3DW no Data Header |
+------------+--------------------+
| Req ID     | 0x20               |
| Tag        | 0xA                |
| Bus        | 0x5                |
| Device     | 0x0                |
| Function   | 0x1                |
| Ext Reg Nr | 0x0                |
| Reg Nr     | 0x0                |
+------------+--------------------+
```

## Usage

```
rtlp-tool [OPTIONS]

Options:
  -i, --input <INPUT>        TLP hex string(s) to parse. May be specified multiple times.
                             Each DWord may optionally be prefixed with 0x/0X.
                             Reads one TLP per line from stdin when omitted.
  -f, --file <FILE>          Read TLP hex strings from a file (one per line)
      --aer                  Scan input for AER TLP headers
                             (matches both 'TLP Header:' and 'HeaderLog:' patterns;
                              associates TLPs with device context only when preceded
                              by lspci-style PCI address lines that start with a PCI
                              address; typical dmesg-style AER lines will not set Source)
      --lspci                Parse lspci -vvv output: extract non-zero HeaderLog entries
                             and annotate each TLP with the device it belongs to
  -c, --count <COUNT>        Process only the first N inputs (default: all)
      --output <FORMAT>      Output format: table (default), json, csv
      --completions <SHELL>  Print shell completion script [bash, zsh, fish, powershell]
      --man                  Print man page in troff format and exit
  -h, --help                 Print help
  -V, --version              Print version
```

### Parse a single TLP

```bash
rtlp-tool -i "04000001 00200a03 05010000 00050100"
```

### 0x-prefixed hex input

Each DWord may optionally carry a `0x` / `0X` prefix — all of the
following forms are accepted and produce identical output:

```bash
# bare hex (standard copy-paste from lspci / dmesg)
rtlp-tool -i "04000001 0000220f 01070000 9eece789"

# fully prefixed
rtlp-tool -i "0x04000001 0x0000220f 0x01070000 0x9eece789"

# mixed
rtlp-tool -i "0x04000001 0000220f 0x01070000 9eece789"
```

### Parse multiple TLPs in one call

Repeat `-i` for each TLP. The tool prints a numbered separator between them:

```bash
rtlp-tool \
  -i "04000001 00200a03 05010000 00050100" \
  -i "4a000001 2001FF00 C281FF10 00000000"

=== TLP #1 ===
+----------+------------------+--------------------+
| TLP Type | ConfType0ReadReq | 3DW no Data Header |
+----------+------------------+--------------------+
...
+------------+--------------------+
| TLP:       | 3DW no Data Header |
+------------+--------------------+
| Req ID     | 0x20               |
| Tag        | 0xA                |
| Bus        | 0x5                |
| Device     | 0x0                |
| Function   | 0x1                |
| Ext Reg Nr | 0x0                |
| Reg Nr     | 0x0                |
+------------+--------------------+

=== TLP #2 ===
+----------+---------+----------------------+
| TLP Type | CplData | 3DW with Data Header |
+----------+---------+----------------------+
...
+-----------------------------+----------------------+
| TLP:                        | 3DW with Data Header |
+-----------------------------+----------------------+
| Compl ID                    | 0x2001               |
| Compl Status                | 0x7                  |
| Byte Count Modified (PCI-X) | 0x1                  |
| Byte Count                  | 0xF00                |
| Req ID                      | 0xC281               |
| Tag                         | 0xFF                 |
| Lower Address               | 0x10                 |
+-----------------------------+----------------------+
```

### Limit with --count

When you have many `-i` inputs but only want to inspect the first few:

```bash
rtlp-tool -i "..." -i "..." -i "..." --count 2
```

### Read TLPs from a file

One hex string per line:

```bash
rtlp-tool -f tlps.txt
rtlp-tool -f tlps.txt --output json
```

### AER log auto-parsing

Pass raw AER kernel messages and let the tool extract every
`TLP Header:` / `HeaderLog:` entry automatically.

```bash
# parse a saved AER dump
rtlp-tool --aer -f aer_dump.txt

# live: filter kernel ring buffer
dmesg | rtlp-tool --aer

# combine with output format
dmesg | rtlp-tool --aer --output json
```

### lspci integration

`--lspci` is purpose-built for `lspci -vvv` output. It scans for
`HeaderLog:` entries, **silently skips all-zero headers** (devices with
no error logged), and annotates every non-zero TLP with the PCIe device
address and name it belongs to.

```bash
# live pipe
lspci -vvv | rtlp-tool --lspci

# from a saved file
rtlp-tool --lspci -f lspci_output.txt

# machine-readable
lspci -vvv | rtlp-tool --lspci --output json
```

Example output:

```
=== TLP #1 ===
+----------+------------------------------------------------------------+--------------------+
| TLP Type | ConfType0ReadReq                                           | 3DW no Data Header |
+----------+------------------------------------------------------------+--------------------+
| Source   | 01:00.0 Non-Volatile memory controller: Phison Electronics |                    |
+----------+------------------------------------------------------------+--------------------+
...
```

### Output format

Three formats are supported via `--output`:

| Format  | Description |
|---------|-------------|
| `table` | Human-readable ASCII tables (default) |
| `json`  | One JSON object per TLP on stdout (ndjson) |
| `csv`   | `index,source,tlp_type,tlp_format,section,key,value` rows |

```bash
rtlp-tool -i "04000001 00200a03 05010000 00050100" --output json
rtlp-tool -f tlps.txt --output csv | column -t -s,
```

### Color output

When stdout is a TTY the table output is automatically colorized to help
spot relevant fields at a glance:

| Color | Meaning |
|-------|---------|
| Blue  | Memory / IO / Atomic TLP types |
| Cyan  | Configuration TLP types |
| Magenta | Message TLP types |
| Green | Completion TLP types |
| Red   | Parse errors |
| Bold red | `Ep` (Error Poison) bit set; non-OK `Compl Status` |
| Yellow | Non-default Traffic Class (`TC`) or ECRC digest (`TD`) field |

Color is suppressed automatically when:
- stdout is not a TTY (e.g. piped to a file or another command), or
- the `NO_COLOR` environment variable is set (any value).

```bash
# disable color explicitly
NO_COLOR=1 rtlp-tool -i "04000001 0000220f 01070000 9eece789"

# color is off automatically when redirecting
rtlp-tool -i "04000001 0000220f 01070000 9eece789" > out.txt
```

### Pipe from stdin

When `-i` and `-f` are omitted the tool reads one TLP hex string per line
from stdin, making it easy to feed AER dumps or scripted output directly:

```bash
# from dmesg
dmesg | grep "TLP Header:" | awk '{print $NF}' | rtlp-tool

# from a file
cat aer_dump.txt | rtlp-tool

# inline heredoc
rtlp-tool <<EOF
04000001 00200a03 05010000 00050100
4a000001 2001FF00 C281FF10 00000000
EOF
```

### Exit codes

| Code | Meaning |
|------|---------|
| `0`  | All TLPs parsed successfully |
| `1`  | One or more TLPs contained an invalid type/format, or input was not valid hex |

Useful for scripting:

```bash
rtlp-tool -i "$header" && echo "TLP is valid" || echo "TLP parse error"
```

### Man page

`--man` prints the full man page in troff format and exits. Use it to
install the page locally or view it directly with `man`:

```bash
# view immediately
rtlp-tool --man | man -l -

# install for the current user (Linux/macOS)
rtlp-tool --man | gzip > ~/.local/share/man/man1/rtlp-tool.1.gz
mandb ~/.local/share/man
```

The pre-built `.deb` package installs the man page automatically, so
`man rtlp-tool` works out of the box on Debian/Ubuntu systems.

### Shell completions

Generate a completion script for your shell and source it:

```bash
# bash
rtlp-tool --completions bash > /etc/bash_completion.d/rtlp-tool

# zsh
rtlp-tool --completions zsh > ~/.zsh/completions/_rtlp-tool

# fish
rtlp-tool --completions fish > ~/.config/fish/completions/rtlp-tool.fish
```

## Installation

### Debian / Ubuntu — pre-built package (recommended)

Download the latest `.deb` from the [Releases page](https://github.com/gotoco/tlp-tool/releases) and install it:

```bash
wget https://github.com/gotoco/tlp-tool/releases/latest/download/rtlp-tool-<VERSION>-amd64.deb
sudo apt install ./rtlp-tool-<VERSION>-amd64.deb
```

Replace `<VERSION>` with the release tag, e.g. `v0.2.0`.

After install the binary is available as `rtlp-tool`:

```bash
rtlp-tool -i "04000001 0000220f 01070000 9eece789"
```

### From crates.io

Requires Rust toolchain installed ([rustup.rs](https://rustup.rs)):

```bash
cargo install rtlp_tool
```

After install the binary is available as `rtlp-tool`:

```bash
rtlp-tool -i "04000001 0000220f 01070000 9eece789"
```

### Build from source

```bash
git clone https://github.com/gotoco/tlp-tool.git
cd tlp-tool
cargo build --release
./target/release/rtlp-tool -i "04000001 0000220f 01070000 9eece789"
```

## Dependencies

 * [rtlp-lib](https://github.com/gotoco/rust_tlplib) — PCI TLP parsing library
 * [clap](https://github.com/clap-rs/clap) — command line argument parser
 * [clap_complete](https://github.com/clap-rs/clap/tree/master/clap_complete) — shell completion script generation
 * [clap_mangen](https://github.com/clap-rs/clap/tree/master/clap_mangen) — man page generation in troff format
 * [prettytable-rs](https://github.com/phsym/prettytable-rs) — terminal table formatting
 * [colored](https://github.com/colored-rs/colored) — terminal color output

## License

Licensed under the [3-Clause BSD License](LICENSE).
