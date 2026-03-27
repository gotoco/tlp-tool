Rust TLP Tool
=============

[![CI](https://github.com/mmpg-x86/tlp-tool/actions/workflows/ci.yml/badge.svg)](https://github.com/mmpg-x86/tlp-tool/actions/workflows/ci.yml)
[![Release](https://github.com/mmpg-x86/tlp-tool/actions/workflows/release.yml/badge.svg)](https://github.com/mmpg-x86/tlp-tool/actions/workflows/release.yml)
[![crates.io](https://img.shields.io/crates/v/rtlp_tool.svg)](https://crates.io/crates/rtlp_tool)

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
+----------+--------------------+--------------------+
| TLP Type | ConfType0ReadReq   | 3DW no Data Header |
+----------+--------------------+--------------------+
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
+----------+--------------------+--------------------+
| TLP Type | ConfType0ReadReq   | 3DW no Data Header |
+----------+--------------------+--------------------+
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
                              address; typical dmesg-style AER lines will not set Source).
                             Flit Mode is auto-detected from the '(Flit)' suffix
                             added by kernels v6.15+ (commit 7e077e6707b3).
      --lspci                Parse lspci -vvv output: extract non-zero HeaderLog entries
                             and annotate each TLP with the device it belongs to.
                             Flit Mode is auto-detected from 'LnkSta2: ... Flit+'.
      --flit                 Force all TLPs to be parsed as PCIe 6.0 flit-mode packets.
                             Normally not needed with --aer or --lspci, which
                             auto-detect Flit Mode. Use for raw hex input on a
                             known Flit Mode link.
  -c, --count <COUNT>        Process only the first N inputs (default: all)
      --output <FORMAT>      Output format: table (default), json (ndjson), csv
      --completions <SHELL>  Print shell completion script [bash, zsh, fish, powershell, elvish]
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

### Flit vs Non-Flit: Which mode should I use?

**The raw TLP hex bytes alone do not tell you whether a link is operating in
Flit Mode or non-Flit Mode.** The framing is a negotiated property of the
link — it is not visible in the TLP header itself.

As a result, the same four bytes decode to completely different packet types
depending on the framing:

| DW0 (hex) | Non-Flit interpretation | Flit interpretation |
|-----------|------------------------|---------------------|
| `04000001` | ConfType0ReadReq | I/O Write |
| `03000001` | ConfType0WriteReq | Memory Read (32-bit) |

Passing the wrong mode flag will produce plausible-looking but incorrect
output with no error or warning.

**About Flit Mode and link speed**

Flit Mode was introduced in the PCIe 6.0 specification. It is mandatory at
64.0 GT/s and supported at all PCIe link speeds. Therefore, link speed alone
does not determine framing: a Flit-capable PCIe 6.x link may be operating
below 64.0 GT/s and still be in Flit Mode. Once a link trains to Flit Mode
it remains in Flit Mode for the duration of the LinkUp state, even if the
negotiated speed changes downward.

**How to determine the negotiated framing**

The authoritative source is the PCIe Capability in config space. Two standard
fields are relevant:

- **Flit Mode Supported** — in the PCIe Capability register
- **Flit Mode Status** — in Link Status 2; reflects the negotiated state

If your `lspci` (pciutils) is recent enough to decode these fields,
`lspci -vv` will show them directly. Otherwise read config space with
`setpci` using capability-relative access, or inspect the raw sysfs file
at `/sys/bus/pci/devices/<BDF>/config`.

On Linux, the kernel caches the negotiated Flit Mode state in
`struct pci_bus` (from Link Status 2) and exposes it through the
`pcie_link_event` tracepoint (`flit_mode` field). There is no generic
`/sys/.../flit_mode` ABI file in the Linux PCI sysfs today.

> **Caveat:** After a DPC trigger or link-down event, Link Status 2 may
> no longer reflect the pre-error state. AER records whether a TLP was
> logged in Flit Mode; DPC does not. For post-mortem diagnostics, rely on
> the kernel-cached value rather than re-reading Link Status 2 live.

**Auto-detection (recommended)**

On kernels v6.15+ (commit `7e077e6707b3`), the kernel appends `(Flit)` to
TLP Header log lines when the link is in Flit Mode. rtlp-tool auto-detects
this suffix — no `--flit` flag needed:

```bash
# Works for both flit and non-flit TLPs, even mixed in the same log
dmesg | rtlp-tool --aer
```

Similarly, `--lspci` auto-detects Flit Mode from `LnkSta2: ... Flit+` in
the `lspci -vv` output:

```bash
# Auto-detects per-device flit mode from LnkSta2
lspci -vvv | rtlp-tool --lspci
```

**Manual override**

For raw hex input, or on older kernels without the `(Flit)` suffix, use
`--flit` to force flit-mode parsing:

```bash
# Non-Flit link (default)
rtlp-tool --aer -f aer_dump.txt

# Force Flit Mode for all TLPs
rtlp-tool --aer --flit -f aer_dump.txt
```

### Flit Mode (PCIe 6.0)

PCIe 6.0 introduced **flit-mode** TLP framing. In flit mode the DW0 encoding
is **completely different** from non-flit framing:

| Field | Non-Flit | Flit (PCIe 6.0) |
|-------|----------|-----------------|
| DW0[7:5] | Fmt (3-bit format) | — |
| DW0[4:0] | Type (5-bit type) | — |
| DW0[7:0] | — | **8-bit flat type code** |
| DW0[15:8] | TC / Attr / LN / TH / … | OHC bitmap |
| DW0[25:16] | Length (10-bit) | Payload length (DWs) |

When using `--aer` or `--lspci`, Flit Mode is auto-detected (see above).
For raw hex input, pass `--flit` to tell rtlp-tool to use flit-mode framing:

```bash
# Flit NOP (type code 0x00)
rtlp-tool --flit -i "00 00 00 00"

# Flit Memory Read 32-bit (type code 0x03)
rtlp-tool --flit -i "03 00 00 01 01 00 0A FF AB CD 12 34"
```

Example output for a flit MemRead32:

```
+----------+----------------------+----------------------+
| TLP Type | Memory Read (32-bit) | Flit Mode (PCIe 6.0) |
+----------+----------------------+----------------------+
+------------+--------+--------+----------------------------+
| Field Name | Offset | Length | Value                      |
|            | (bits) | (bits) |                            |
+------------+--------+--------+----------------------------+
| Type Code  | 0      | 8      | 0x03  (Memory Read (32-bit)) |
| OHC        | 8      | 8      | 0x00                       |
| OHC Count  | -      | -      | 0 extension DW(s)          |
| Length     | 16     | 10     | 1                          |
+------------+--------+--------+----------------------------+
+-----------+----------------------+
| Flit TLP: | Flit Mode (PCIe 6.0) |
+-----------+----------------------+
| DW0       | 0x03000001           |
| DW1       | 0x01000AFF           |
| DW2       | 0xABCD1234           |
+-----------+----------------------+
```

JSON output gains a `flit_mode` boolean field:

```bash
rtlp-tool --flit -i "03 00 00 01 01 00 0A FF AB CD 12 34" --output json
# → {"index":1,"tlp_type":"Memory Read (32-bit)","tlp_format":"Flit Mode (PCIe 6.0)","flit_mode":true,...}
```

Without `--flit` the tool defaults to non-flit mode for raw hex input —
all existing scripts and pipelines continue to work unchanged. When using
`--aer` or `--lspci`, Flit Mode is auto-detected per-TLP from kernel/lspci
markers, so `--flit` is typically not needed.

**Supported flit type codes** (rtlp-lib 0.5.0):

| Code | Type | Notes |
|------|------|-------|
| `0x00` | NOP | |
| `0x03` | Memory Read (32-bit) | |
| `0x22` | UIO Memory Read (64-bit) | |
| `0x30` | Message routed to RC | |
| `0x40` | Memory Write (32-bit) | |
| `0x42` | I/O Write | OHC byte **must** be `0x01` (mandatory OHC) |
| `0x44` | Config Type 0 Write | OHC byte **must** be `0x01` (mandatory OHC) |
| `0x4C` | FetchAdd AtomicOp (32-bit) | |
| `0x4E` | CompareSwap AtomicOp (32-bit) | |
| `0x5B` | Deferrable Memory Write (32-bit) | |
| `0x61` | UIO Memory Write (64-bit) | |
| `0x70` | Message with Data routed to RC | |
| `0x8D` | Local TLP Prefix | |

> **Note:** Types marked "mandatory OHC" require byte 1 of DW0 to have bit 0 set (`OHC=0x01`).
> rtlp-lib validates this and returns a `MissingMandatoryOhc` error if the bit is absent.
> Example: `rtlp-tool --flit -i "42 01 00 01 01 00 0A FF AB CD 12 34 DE AD BE EF 00 00 00 00"`

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
| `0`  | All TLPs parsed successfully (also returned by `--help`, `--version`, `--man`, `--completions`) |
| `1`  | One or more TLPs had an invalid type or format |
| `1`  | Input contained non-hex characters |
| `1`  | Specified file could not be opened (`-f <FILE>`) |
| `1`  | `--aer` mode: no `TLP Header:` / `HeaderLog:` lines found in input |
| `1`  | `--lspci` mode: no non-zero `HeaderLog:` entries found in input |
| `1`  | No input provided at all (empty stdin, no `-i`, no `-f`) |

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

# powershell
rtlp-tool --completions powershell >> $PROFILE

# elvish
rtlp-tool --completions elvish > ~/.config/elvish/lib/rtlp-tool.elv
```

## Installation

All release artifacts are built automatically by GitHub Actions and attached
to every [GitHub Release](https://github.com/mmpg-x86/tlp-tool/releases).
Replace `<VERSION>` with the version number (without the `v` prefix),
e.g. `0.5.0`.

### Debian / Ubuntu (.deb)

```bash
wget https://github.com/mmpg-x86/tlp-tool/releases/latest/download/rtlp-tool-<VERSION>-amd64.deb
sudo apt install ./rtlp-tool-<VERSION>-amd64.deb
```

The `.deb` includes the binary, man page, and README. After install:

```bash
rtlp-tool -i "04000001 0000220f 01070000 9eece789"
man rtlp-tool
```

### Fedora / RHEL / openSUSE (.rpm)

```bash
wget https://github.com/mmpg-x86/tlp-tool/releases/latest/download/rtlp-tool-<VERSION>-x86_64.rpm
sudo rpm -i rtlp-tool-<VERSION>-x86_64.rpm
# or with dnf:
sudo dnf install ./rtlp-tool-<VERSION>-x86_64.rpm
```

### macOS — pre-built binary

Download the tarball for your architecture:

```bash
# Apple Silicon (M1 / M2 / M3)
curl -Lo rtlp-tool.tar.gz \
  https://github.com/mmpg-x86/tlp-tool/releases/latest/download/rtlp-tool-macos-aarch64.tar.gz

# Intel Mac
curl -Lo rtlp-tool.tar.gz \
  https://github.com/mmpg-x86/tlp-tool/releases/latest/download/rtlp-tool-macos-x86_64.tar.gz

tar -xf rtlp-tool.tar.gz
sudo mv rtlp-tool /usr/local/bin/
rtlp-tool -i "04000001 0000220f 01070000 9eece789"
```

### FreeBSD — pre-built binary

```bash
fetch https://github.com/mmpg-x86/tlp-tool/releases/latest/download/rtlp-tool-freebsd-x86_64.tar.gz
tar -xf rtlp-tool-freebsd-x86_64.tar.gz
sudo mv rtlp-tool /usr/local/bin/
```

### Linux — static binary (any distro)

The `linux-x86_64` and `linux-aarch64` tarballs are statically linked
(musl libc) and run on any modern Linux without extra dependencies:

```bash
# x86_64
curl -Lo rtlp-tool.tar.gz \
  https://github.com/mmpg-x86/tlp-tool/releases/latest/download/rtlp-tool-linux-x86_64.tar.gz

# aarch64 (Raspberry Pi 4, AWS Graviton, etc.)
curl -Lo rtlp-tool.tar.gz \
  https://github.com/mmpg-x86/tlp-tool/releases/latest/download/rtlp-tool-linux-aarch64.tar.gz

tar -xf rtlp-tool.tar.gz
sudo mv rtlp-tool /usr/local/bin/
```

### Windows — pre-built binary

Download `rtlp-tool-windows-x86_64.zip` from the
[Releases page](https://github.com/mmpg-x86/tlp-tool/releases), extract
`rtlp-tool.exe`, and place it somewhere on your `PATH`.

### From crates.io

Requires Rust toolchain installed ([rustup.rs](https://rustup.rs)):

```bash
cargo install rtlp_tool
```

### Build from source

```bash
git clone https://github.com/mmpg-x86/tlp-tool.git
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
