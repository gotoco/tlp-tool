# CR-001: Enhanced Config TLP Decoding, DWord Endianness Swap & Comma-Tolerant Input

| Field        | Value                                                 |
|--------------|-------------------------------------------------------|
| **ID**       | CR-001                                                |
| **Date**     | 2026-04-16                                            |
| **Author**   | Maciej Grochowski                                     |
| **Status**   | Draft                                                 |
| **Affects**  | `rtlp-tool` (CLI), `rtlp-lib` (if body changes needed) |
| **Version**  | Target: v0.6.0                                        |

---

## 1. Motivation

### 1.1 Configuration TLP output is too raw

The current output for `CfgRd0`, `CfgRd1`, `CfgWr0`, `CfgWr1` TLPs shows low-level
field extractions that require the user to manually compute the actual config-space
register offset and look up what it means:

```
+------------+----------------------+
| TLP:       | 3DW with Data Header |
+------------+----------------------+
| Req ID     | 0x0                  |
| Tag        | 0x21                 |
| Bus        | 0x4                  |
| Device     | 0x0                  |
| Function   | 0x1                  |
| Ext Reg Nr | 0x0                  |
| Reg Nr     | 0x1                  |
+------------+----------------------+
```

**Problems:**

1. **No BDF (Bus:Device.Function) in standard notation** -- the user must mentally
   assemble `04:00.1` from three separate fields.

2. **No computed register offset** -- the user must manually calculate
   `(Ext Reg Nr << 8) | (Reg Nr << 2)` to determine the actual config-space byte
   offset. In the example above: `(0x0 << 8) | (0x1 << 2) = 0x004` -- this is the
   **Command/Status** register, but nothing in the output says so.

3. **No register name** -- standard PCIe/PCI config-space registers at well-known
   offsets (0x00-0x3F for Type 0/1 headers) have defined names in the PCI spec.
   The tool should display them when the offset falls within the standard header.

4. **No First/Last DW Byte Enable** -- config TLPs carry byte enables that indicate
   which bytes within the target DWORD are actually being read/written. These are
   currently not shown for config requests (they ARE shown for memory requests).

5. **No data payload display for CfgWr0/CfgWr1** -- write TLPs carry a data DWORD
   (DW3) containing the value being written. The tool currently does not display it,
   so the user cannot see *what* is being written to the register.

### 1.2 No support for DWord byte-swap (endianness)

PCIe TLP headers are defined as **big-endian** (network byte order) per the PCIe
Base Specification. However, when TLP header bytes are captured by a little-endian
CPU (e.g., MIPS in LE mode, some ARM configurations, or certain PCIe IP register
dumps), the bytes within each 32-bit DWord are reversed.

Currently, there is no way to tell `rtlp-tool` that the input DWords need byte-swapping.
The user must manually reverse each DWord before feeding it to the tool, which is
error-prone and tedious.

**Real-world example (MIPS LE):**

| Raw from CPU          | After DWord byte-swap | Correct decode |
|-----------------------|-----------------------|----------------|
| `0x01000044`          | `0x44000001`          | CfgWr0, Length=1 |
| `0x03210000`          | `0x00002103`          | ReqID=0x0000, Tag=0x21, FDWBE=0x3 |

Without the swap, the tool decodes this as `MemReadLockReq` with `Length=68` -- a
nonsensical result that wastes debugging time.

**Real-world example #2: 64-bit Memory Read across NTB (MIPS LE)**

A MIPS root complex reading 8 bytes through a Non-Transparent Bridge:

```
Raw log:    0x02100020  0xFF200000  0x04000000  0x00000040
Corrected:  0x20001002  0x000020FF  0x00000004  0x40000000
            ─── DW0 ──  ─── DW1 ──  ─── DW2 ──  ─── DW3 ──
```

After byte-swap, this decodes as:
- **MRd64** (4DW, no data) — `Fmt=001`, `Type=00000`, Length=2 (8 bytes)
- **Requester ID** = `0x0000` (Bus 0, Dev 0, Fn 0 — the MIPS CPU / root complex)
- **Tag** = `0x20`, **Last DW BE** = `0xF`, **First DW BE** = `0xF`
- **Address** = `0x00000004_40000000` (NTB-mapped window to remote memory)

Without `--swap`, this produces garbage. With `--swap`, the tool correctly identifies
a 64-bit memory read from the root complex across the NTB.

### 1.3 Input with commas is rejected

When users copy TLP hex values from CSV files, spreadsheets, or comma-separated
log outputs, the input often contains commas between DWords:

```
0x02100020, 0xff200000, 0x04000000, 0x00000040
```

Currently, `rtlp-tool` fails with `"not valid hex"` because the input sanitizer
(`remove_whitespace()`) only strips whitespace and `0x` prefixes — commas pass
through to the hex parser which rejects them. The user must manually remove all
commas before pasting, which is tedious and breaks the copy-paste workflow.

---

## 2. Requirements

### REQ-1: DWord Byte-Swap Flag (`--swap`)

| Aspect       | Detail |
|--------------|--------|
| **Flag**     | `--swap` (long), `-s` (short) |
| **Type**     | Boolean flag (no value) |
| **Default**  | Off (input assumed big-endian, per PCIe spec) |
| **Behavior** | When set, reverse the byte order within each 32-bit DWord of the raw input *before* any TLP parsing takes place. |
| **Scope**    | Applies to all input modes: `-i`, `-f`, `--aer`, `--lspci`, `stdin` |
| **Flit compat** | Must work with both non-flit and `--flit` modes |

**Algorithm** (applied per DWord):
```
input:  [b0, b1, b2, b3]   (as received from the little-endian source)
output: [b3, b2, b1, b0]   (big-endian, as PCIe spec defines TLP headers)
```

If the total input byte count is not a multiple of 4, emit an error:
```
error: --swap requires input length to be a multiple of 4 bytes (got N bytes)
```

**CLI help text:**
```
--swap, -s   Byte-swap each 32-bit DWord before parsing.
             Use when input was captured on a little-endian CPU
             (e.g. MIPS LE, some ARM) and the bytes within each
             DWord are reversed relative to PCIe wire order.
```

### REQ-2: Enhanced Configuration TLP Body Output

Replace the current `collect_cfg_req()` output with a richer display for all four
config TLP types: `CfgRd0`, `CfgRd1`, `CfgWr0`, `CfgWr1`.

#### REQ-2a: BDF in Standard Notation

Add a `Target BDF` field that combines Bus, Device, and Function into the standard
`BB:DD.F` notation (hex, zero-padded):

```
| Target BDF  | 04:00.1 |
```

Keep the individual `Bus`, `Device`, `Function` fields as well for completeness,
but place `Target BDF` first as the primary human-readable identifier.

#### REQ-2b: Computed Register Offset

Add a `Register Offset` field that computes the actual config-space byte offset:

```
offset = (Ext Reg Nr << 8) | (Reg Nr << 2)
```

Display as hex with `0x` prefix:

```
| Register Offset | 0x004 |
```

Keep `Ext Reg Nr` and `Reg Nr` as raw fields below it for reference.

#### REQ-2c: Standard Config Register Name Lookup

For offsets in the standard PCI/PCIe Type 0 configuration header (`0x00`--`0x3F`),
display the well-known register name. This covers the most commonly accessed
registers during device enumeration and initialization.

**Type 0 Header Register Map (mandatory):**

| Offset | Size | Name |
|--------|------|------|
| 0x00   | 2    | Vendor ID |
| 0x02   | 2    | Device ID |
| 0x04   | 2    | Command |
| 0x06   | 2    | Status |
| 0x08   | 1    | Revision ID |
| 0x09   | 3    | Class Code |
| 0x0C   | 1    | Cache Line Size |
| 0x0D   | 1    | Latency Timer |
| 0x0E   | 1    | Header Type |
| 0x0F   | 1    | BIST |
| 0x10   | 4    | BAR 0 |
| 0x14   | 4    | BAR 1 |
| 0x18   | 4    | BAR 2 |
| 0x1C   | 4    | BAR 3 |
| 0x20   | 4    | BAR 4 |
| 0x24   | 4    | BAR 5 |
| 0x28   | 4    | Cardbus CIS Pointer |
| 0x2C   | 2    | Subsystem Vendor ID |
| 0x2E   | 2    | Subsystem ID |
| 0x30   | 4    | Expansion ROM Base Address |
| 0x34   | 1    | Capabilities Pointer |
| 0x3C   | 1    | Interrupt Line |
| 0x3D   | 1    | Interrupt Pin |
| 0x3E   | 1    | Min_Gnt |
| 0x3F   | 1    | Max_Lat |

**Type 1 Header Register Map (for CfgRd1/CfgWr1, bridges):**

| Offset | Size | Name |
|--------|------|------|
| 0x00   | 2    | Vendor ID |
| 0x02   | 2    | Device ID |
| 0x04   | 2    | Command |
| 0x06   | 2    | Status |
| 0x08   | 1    | Revision ID |
| 0x09   | 3    | Class Code |
| 0x0C   | 1    | Cache Line Size |
| 0x0D   | 1    | Latency Timer |
| 0x0E   | 1    | Header Type |
| 0x0F   | 1    | BIST |
| 0x10   | 4    | BAR 0 |
| 0x14   | 4    | BAR 1 |
| 0x18   | 1    | Primary Bus Number |
| 0x19   | 1    | Secondary Bus Number |
| 0x1A   | 1    | Subordinate Bus Number |
| 0x1B   | 1    | Secondary Latency Timer |
| 0x1C   | 1    | I/O Base |
| 0x1D   | 1    | I/O Limit |
| 0x1E   | 2    | Secondary Status |
| 0x20   | 2    | Memory Base |
| 0x22   | 2    | Memory Limit |
| 0x24   | 2    | Prefetchable Memory Base |
| 0x26   | 2    | Prefetchable Memory Limit |
| 0x28   | 4    | Prefetchable Base Upper 32 |
| 0x2C   | 4    | Prefetchable Limit Upper 32 |
| 0x30   | 2    | I/O Base Upper 16 |
| 0x32   | 2    | I/O Limit Upper 16 |
| 0x34   | 1    | Capabilities Pointer |
| 0x38   | 4    | Expansion ROM Base Address |
| 0x3C   | 1    | Interrupt Line |
| 0x3D   | 1    | Interrupt Pin |
| 0x3E   | 2    | Bridge Control |

**Display logic:**

- For `CfgRd0` / `CfgWr0`: use the Type 0 register map.
- For `CfgRd1` / `CfgWr1`: use the Type 1 register map.
- The register lookup is based on which DWORD the offset falls into (offset aligned
  down to 4-byte boundary), then the First/Last DW BE determines which bytes within
  that DWORD are targeted.
- If the offset is `>= 0x40` (capabilities/extended config space), display:
  ```
  | Register Name | (capabilities region - offset 0xNN) |
  ```
- If the offset is `>= 0x100` (PCIe extended config space), display:
  ```
  | Register Name | (extended config space - offset 0xNNN) |
  ```

#### REQ-2d: Byte Enable Display

Add `First DW BE` and `Last DW BE` fields to the config TLP body output, as is
already done for memory requests. Display as hex.

Additionally, decode the First DW BE into a human-readable byte-lane indicator:
```
| First DW BE  | 0x3 (bytes 0-1) |
| Last DW BE   | 0x0 (none)      |
```

#### REQ-2e: Data Payload for Config Writes

For `CfgWr0` and `CfgWr1` TLPs, display the DW3 data payload:

```
| Data          | 0x00000007 |
```

The data DWORD is at byte offset 12-15 in the raw TLP (DW3). For config writes,
this is the value being written to the target register.

If the register name is known (REQ-2c), also display a combined summary line:

```
| Operation     | Write 0x0007 to Command register at 04:00.1 |
```

For config reads, display a summary:
```
| Operation     | Read Command register at 04:00.1 |
```

### REQ-4: Strip Commas from Hex Input

| Aspect       | Detail |
|--------------|--------|
| **Behavior** | Strip commas (`,`) from the raw hex input during sanitization, alongside the existing whitespace and `0x` prefix stripping. |
| **Scope**    | All input modes: `-i`, `-f`, `--aer`, `--lspci`, `stdin` |
| **No flag**  | This is always-on — commas are never valid hex, so stripping them is safe and unambiguous. |

**Accepted input formats (all equivalent):**
```
0x02100020 0xff200000 0x04000000 0x00000040       (current - works)
0x02100020, 0xff200000, 0x04000000, 0x00000040    (CSV-style - NEW)
0x02100020,0xff200000,0x04000000,0x00000040       (no spaces - NEW)
02100020,ff200000,04000000,00000040                (bare hex + commas - NEW)
```

**Implementation:** Modify `Config::remove_whitespace()` (src/main.rs, ~line 272)
to treat commas as token separators alongside whitespace:

```rust
fn remove_whitespace(s: &str) -> String {
    // Split on whitespace AND commas, strip 0x/0X prefix, concatenate
    s.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|tok| !tok.is_empty())
        .map(|tok| {
            tok.strip_prefix("0x")
                .or_else(|| tok.strip_prefix("0X"))
                .unwrap_or(tok)
        })
        .collect()
}
```

### REQ-5: Updated Output for All Formats

All three output formats (table, JSON, CSV) must include the new fields.

#### REQ-5a: Table Output

Example for a CfgWr0 TLP:

```
+----------+------------------+----------------------+
| TLP Type | ConfType0WriteReq | 3DW with Data Header |
+----------+------------------+----------------------+
+------------+--------+--------+-------+
| Field Name | Offset | Length | Value |
|            | (bits) | (bits) |       |
+------------+--------+--------+-------+
| Fmt        | 0      | 3      | 2     |
| Type       | 3      | 5      | 4     |
| ...        | ...    | ...    | ...   |
| Length     | 22     | 10     | 1     |
+------------+--------+--------+-------+
+-------------------+----------------------------------------------+
| TLP:              | 3DW with Data Header                         |
+-------------------+----------------------------------------------+
| Req ID            | 0x0                                          |
| Tag               | 0x21                                         |
| First DW BE       | 0x3 (bytes 0-1)                              |
| Last DW BE        | 0x0 (none)                                   |
| Target BDF        | 04:00.2                                      |
| Bus               | 0x4                                          |
| Device            | 0x0                                          |
| Function          | 0x2                                          |
| Register Offset   | 0x004                                        |
| Register Name     | Command                                      |
| Ext Reg Nr        | 0x0                                          |
| Reg Nr            | 0x1                                          |
| Data              | 0x00000007                                   |
| Operation         | Write 0x0007 to Command register at 04:00.2  |
+-------------------+----------------------------------------------+
```

#### REQ-5b: JSON Output

Add new keys to the `body` object:

```json
{
  "body": {
    "Req ID": "0x0",
    "Tag": "0x21",
    "First DW BE": "0x3",
    "Last DW BE": "0x0",
    "Target BDF": "04:00.2",
    "Bus": "0x4",
    "Device": "0x0",
    "Function": "0x2",
    "Register Offset": "0x004",
    "Register Name": "Command",
    "Ext Reg Nr": "0x0",
    "Reg Nr": "0x1",
    "Data": "0x00000007",
    "Operation": "Write 0x0007 to Command register at 04:00.2"
  }
}
```

#### REQ-5c: CSV Output

New keys appear as additional `body` rows:
```
1,,ConfType0WriteReq,3DW with Data Header,body,Target BDF,04:00.2
1,,ConfType0WriteReq,3DW with Data Header,body,Register Offset,0x004
1,,ConfType0WriteReq,3DW with Data Header,body,Register Name,Command
...
```

---

## 3. Implementation Notes

### 3.1 DWord Byte-Swap (REQ-1)

The swap should be applied at the earliest point, in `Config::new()`, before the
bytes are stored. This ensures all downstream code sees correctly-ordered bytes.

**Suggested location:** `Config::new()` (src/main.rs, ~line 304)

```rust
// After convert_to_vec, before pushing to inputs:
if swap {
    for chunk in bytes.chunks_exact_mut(4) {
        chunk.reverse();
    }
}
```

The `--swap` flag needs to be threaded through:
1. `Args` struct -- add `swap: bool` field with clap attributes
2. `Config` struct -- add `swap: bool` field
3. `Config::new()` -- accept `swap` parameter, apply per-DWord reversal

### 3.2 Config TLP Body Enhancement (REQ-2)

**Suggested location:** Replace `collect_cfg_req()` (src/main.rs, ~line 418)

The function currently returns `Vec<(String, String)>`. The same return type can be
used; just add more entries to the vector.

Key changes:
1. The function needs to know whether this is a Type 0 or Type 1 config TLP (to select
   the correct register map). This requires passing the `TlpType` to the function,
   or splitting into `collect_cfg_type0_req()` / `collect_cfg_type1_req()`.

2. For write TLPs, access DW3 from the raw packet data. Currently `collect_cfg_req()`
   only receives a `&TlpPacket`. The raw bytes may need to be passed as well (similar
   to how `collect_tlp()` already receives `raw_bytes`).

3. The register name lookup table should be a `const` array or `match` expression --
   no external dependencies needed.

### 3.3 Byte Enable Decoding

Helper function to convert a 4-bit byte enable to a human-readable string:

```rust
fn decode_byte_enable(be: u8) -> &'static str {
    match be & 0xF {
        0x0 => "none",
        0x1 => "byte 0",
        0x2 => "byte 1",
        0x3 => "bytes 0-1",
        0x4 => "byte 2",
        0x5 => "bytes 0 and 2",
        0x6 => "bytes 1-2",
        0x7 => "bytes 0-2",
        0x8 => "byte 3",
        0x9 => "bytes 0 and 3",
        0xA => "bytes 1 and 3",
        0xB => "bytes 0-1 and 3",
        0xC => "bytes 2-3",
        0xD => "bytes 0 and 2-3",
        0xE => "bytes 1-3",
        0xF => "bytes 0-3",
        _   => unreachable!(),
    }
}
```

### 3.4 Comma Stripping (REQ-4)

The change is minimal and entirely within `Config::remove_whitespace()`. The current
implementation uses `s.split_whitespace()` which only splits on Unicode whitespace.
Changing to `s.split(|c: char| c.is_whitespace() || c == ',')` plus
`.filter(|tok| !tok.is_empty())` handles all comma patterns.

**Processing order** (important for `--swap` interaction):
```
raw input string
  → remove_whitespace()     [strips commas, whitespace, 0x prefixes]
  → convert_to_vec()        [hex string → Vec<u8>]
  → byte-swap if --swap     [reverse each 4-byte chunk]
  → store in Config.inputs  [ready for TLP parsing]
```

Commas are stripped before byte-swap, so `-s -i "0x02100020, 0xFF200000, ..."` works
correctly: commas are removed, hex is parsed to bytes, then each DWord is swapped.

### 3.5 rtlp-lib Dependency

The current `rtlp-lib 0.5.0` `new_conf_req()` returns a struct with:
- `req_id()`, `tag()`, `bus_nr()`, `dev_nr()`, `func_nr()`, `ext_reg_nr()`, `reg_nr()`

It does **not** expose:
- First/Last DW byte enables for config requests
- DW3 data payload for config writes

**Options:**
- **(A) Extract from raw bytes in rtlp-tool** -- The byte enables are at DW1[3:0]
  (First DW BE) and DW1[7:4] (Last DW BE), same layout as memory requests. DW3 data
  is at raw bytes [12..16]. This avoids changing rtlp-lib.
- **(B) Extend rtlp-lib** -- Add `fdwbe()`, `ldwbe()`, and optionally `data_dw()`
  to the config request struct. Cleaner but requires a lib release.

**Recommendation:** Option (A) for initial implementation -- extract directly from raw
bytes in `collect_cfg_req()`. This keeps the change self-contained in rtlp-tool. A
follow-up PR can move the logic into rtlp-lib if desired.

Raw byte layout for 3DW Config Request (PCIe Base Spec):
```
DW0 [bytes  0- 3]: Fmt | Type | TC | ... | Length
DW1 [bytes  4- 7]: Requester ID [31:16] | Tag [15:8] | Last DW BE [7:4] | First DW BE [3:0]
DW2 [bytes  8-11]: Bus [31:24] | Device [23:19] | Function [18:16] | Rsvd [15:12] | Ext Reg Nr [11:8] | Reg Nr [7:2] | Rsvd [1:0]
DW3 [bytes 12-15]: Data (CfgWr only)
```

---

## 4. Test Plan

### 4.1 Endianness Swap Tests

| Test | Input | Expected |
|------|-------|----------|
| `--swap` CfgWr0 (MIPS LE) | `-s -i "0x01000044 0x03210000 0x04000104 0x00000007"` | Decodes as `ConfType0WriteReq`, Length=1, Target BDF `04:00.2`, Reg=Command |
| `--swap` MRd64 (MIPS LE NTB) | `-s -i "0x02100020 0xFF200000 0x04000000 0x00000040"` | Decodes as `MemReadReq`, 4DW, Length=2, ReqID=0x0000, Addr High=0x00000004, Addr Low=0x40000000, FDWBE=0xF, LDWBE=0xF |
| `--swap` identity | `0x44000001 0x00002103 0x04010004 0x07000000` without `--swap` | Same result as CfgWr0 swap test above |
| `--swap` with non-multiple-of-4 | `0x440000` with `--swap` | Error about input length |
| `--swap` with `--aer` | AER log from LE system with `--swap` | Correct TLP decode |
| `--swap` with `--flit` | Flit-mode DWords byte-swapped with `--swap` | Correct flit decode |
| `--swap` with `--lspci` | lspci output from LE system with `--swap` | Correct TLP decode |
| `--swap` + commas | `-s -i "0x02100020, 0xFF200000, 0x04000000, 0x00000040"` | Same as MRd64 test above (commas stripped, then swapped) |
| Double swap is identity | Input with `--swap` → output matches same input without `--swap` when pre-swapped | Consistent |

### 4.2 Enhanced Config TLP Tests

| Test | Input | Expected Output Contains |
|------|-------|--------------------------|
| CfgRd0 basic | `04000001 0000220f 01070000 ...` | `Target BDF: 01:00.7`, `Register Offset: 0x000`, `Register Name: Vendor ID` |
| CfgWr0 with data | Byte-swapped example from motivation | `Target BDF: 04:00.2`, `Register Name: Command`, `Data: 0x00000007`, `Operation: Write 0x0007 to Command register` |
| CfgRd1 bridge | Type 1 config read to offset 0x18 | `Register Name: Primary Bus Number` |
| CfgWr1 bridge | Type 1 config write to offset 0x3E | `Register Name: Bridge Control` |
| Capabilities region | Config read at offset 0x40+ | `Register Name: (capabilities region - offset 0xNN)` |
| Extended config | Config read at offset 0x100+ | `Register Name: (extended config space - offset 0xNNN)` |
| First DW BE decode | FDWBE=0x3 | `First DW BE: 0x3 (bytes 0-1)` |
| Last DW BE decode | LDWBE=0x0 | `Last DW BE: 0x0 (none)` |
| JSON output | CfgWr0 with `--output json` | `"Target BDF"`, `"Register Name"`, `"Data"`, `"Operation"` keys present |
| CSV output | CfgWr0 with `--output csv` | Rows with `body,Target BDF,...` and `body,Register Name,...` |

### 4.3 Comma-Separated Input Tests

| Test | Input | Expected |
|------|-------|----------|
| CSV-style with spaces | `-i "0x04000001, 0x0000220f, 0x01070000, 0x9eece789"` | Decodes as `ConfType0ReadReq` (same as without commas) |
| CSV-style no spaces | `-i "0x04000001,0x0000220f,0x01070000,0x9eece789"` | Same as above |
| Bare hex with commas | `-i "04000001,0000220f,01070000,9eece789"` | Same as above |
| Mixed separators | `-i "0x04000001, 0000220f,0x01070000 9eece789"` | Same as above (commas, spaces, mixed prefixes all tolerated) |
| Commas with `--swap` | `-s -i "0x02100020, 0xFF200000, 0x04000000, 0x00000040"` | Decodes as `MemReadReq` 4DW (commas stripped first, then DWords swapped) |
| Commas in `--aer` mode | AER log where HeaderLog values are comma-separated | Correct extraction and decode |
| Commas in file input | `-f` with file containing comma-separated hex per line | Each line parsed correctly |
| Trailing comma | `-i "04000001, 0000220f, 01070000, 9eece789,"` | Trailing comma ignored (empty token after split is filtered) |
| JSON output with commas | `-i "0x04000001, 0x0000220f, 0x01070000, 0x9eece789" --output json` | Valid JSON, identical to non-comma input |

### 4.4 Backward Compatibility

| Test | Expectation |
|------|-------------|
| All existing tests pass without modification | No breaking changes to existing output fields |
| Without `--swap`, behavior is identical | No change to default parsing path |
| Memory/Completion/Message TLP output unchanged | Only config TLPs affected |

---

## 5. Affected Files

| File | Change Type | Description |
|------|-------------|-------------|
| `src/main.rs` | Modified | Add `--swap` flag to `Args`, byte-swap logic in `Config::new()`, enhanced `collect_cfg_req()`, register name lookup tables, byte enable decoder |
| `tests/cli.rs` | Modified | New test cases per Section 4 |
| `tests/fixtures/` | New files | Test fixtures for `--swap` scenarios |
| `README.md` | Modified | Document `--swap` flag, updated config TLP output examples |

---

## 6. Out of Scope (Future Work)

The following are explicitly **not** part of this change request but are noted for
potential future enhancements:

1. **PCIe Capability ID decode** -- When the register offset falls in the
   capabilities region (0x40+), walk the capability linked list to identify specific
   capabilities (MSI, MSI-X, PCIe, Power Management, etc.). This requires knowing
   the device's capability layout, which is not available from a single TLP header.

2. **Extended Capability decode** -- Similar to above for the 0x100+ region.

3. **Automatic endianness detection** -- Heuristic-based detection of whether the
   input needs byte-swapping (e.g., if Fmt/Type fields decode to an invalid or
   extremely rare TLP type, try the swapped version). This is fragile and could
   mask real errors, so an explicit flag is preferred.

4. **Data payload decode** -- For CfgWr TLPs where the register is known (e.g.,
   Command register), decode the individual bits of the data value (e.g.,
   "IO Space=1, Memory Space=1, Bus Master=1"). This adds significant complexity
   for limited benefit in a generic tool.

5. **rtlp-lib enhancement** -- Move byte enable, data extraction, register name
   lookup, and BDF formatting into the library for reuse by other consumers.

---

## 7. References

- PCIe Base Specification, Rev 5.0 / 6.0 -- Configuration Request TLP format
  (Section 2.2.8, Table 2-15/2-16)
- PCI Local Bus Specification, Rev 3.0 -- Type 0/1 Configuration Space Header
  (Section 6.1)
- rtlp-lib 0.5.0 API: `new_conf_req()`, `ConfReq` struct
- Original issue: MIPS LE byte-order mismatch producing incorrect TLP decode
