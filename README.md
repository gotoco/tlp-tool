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
This problematic TLP Header "04000001 0000220f 01070000 9eece789" can be easily parsed by rtlp_tool

```bash
rtlp_tool -i "04000001 0000220f 01070000 9eece789"
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

TLP Header `04000001 00200a03 05010000 00050100` can be parsed by rtlp_tool via:

```bash
rtlp_tool -i "04000001 00200a03 05010000 00050100"
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

## Dependencies

 * Rust TLP Lib: rust_tlplib
 * Command Line Argument Parser for Rust: clap
 * Rust Pretty Table: prettytable-rs

## License

Licensed under:

 * The 3-Clause BSD License
