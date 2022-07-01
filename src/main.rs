use std::convert::TryFrom;
use std::result::Result;

use rtlp_lib::{TlpPacket, TlpPacketHeader, TlpType, TlpFmt};
use rtlp_lib::{new_mem_req, new_conf_req, new_cmpl_req, new_msg_req};

use clap::Parser;

#[macro_use] extern crate prettytable;
use prettytable::{Table}; //, Row, Cell};
use prettytable::format;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    input: String,

    #[clap(short, long, default_value_t = 1)]
    count: u8,
}

struct Config {
    input: Vec<u8>,
}

struct TlpTool {
    config: Config,
}

impl TlpTool {
    fn new(cfg: Config) -> TlpTool {

        TlpTool { config: cfg }
    }

    fn display_tlp_header(&self, header: &TlpPacketHeader) {
		let mut table = Table::new();
		table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
		table.add_row(row!["Fmt", "0", "3", header.get_format()]);
		table.add_row(row!["Type", "3", "5", header.get_type()]);
		table.add_row(row!["T9", "8", "1", header.get_t9()]);
		table.add_row(row!["TC", "9", "3", header.get_tc()]);
		table.add_row(row!["T8", "12", "1", header.get_t8()]);
		table.add_row(row!["Attr_b2", "13", "1", header.get_attr_b2()]);
		table.add_row(row!["LN", "14", "1", header.get_ln()]);
		table.add_row(row!["TH", "15", "1", header.get_th()]);
		table.add_row(row!["Td", "16", "1", header.get_td()]);
		table.add_row(row!["Ep", "17", "1", header.get_ep()]);
		table.add_row(row!["Attr", "18", "2", header.get_attr()]);
		table.add_row(row!["AT", "20", "2", header.get_at()]);
		table.add_row(row!["Length", "22", "10", header.get_length()]);
		table.set_titles(row!["Field Name", "Offset\n(bits)", "Length\n(bits)", "Value"]);

		table.printstd();
	}

    fn display_tlp_type(&self, tlp: &TlpPacket) {
		let mut table = Table::new();
		table.add_row(row!["TLP Type", tlp.get_tlp_type(), tlp.get_tlp_format()]);

		table.printstd();
	}

	fn display_mem_req(&self, tlp: &TlpPacket) {
		let tlpf = tlp.get_tlp_format();
		let mut table = Table::new();

        if let Ok(tlpf1) = TlpFmt::try_from(tlpf) {
            let addr_desc;
            let mr = new_mem_req(tlp.get_data(), &tlpf1);
            let addr = mr.address();
            let reqid = mr.req_id();
            let tag = mr.tag();
            let ldwbe = mr.ldwbe();
            let fdwbe = mr.fdwbe();

            table.set_titles(row!["TLP: ", tlp.get_tlp_format()]);
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.add_row(row!["Req ID", format!("{reqid:#X}")]);
            table.add_row(row!["Tag", format!("{tag:#X}")]);
            table.add_row(row!["Last DW BE", format!("{ldwbe:#X}")]);
            table.add_row(row!["First DW BE", format!("{fdwbe:#X}")]);
            
            match tlpf1 {
            	TlpFmt::NoDataHeader3DW | TlpFmt::WithDataHeader3DW => addr_desc = "Address (32b)",
            	TlpFmt::NoDataHeader4DW | TlpFmt::WithDataHeader4DW => addr_desc = "Address (64b)",
            	TlpFmt::TlpPrefix => addr_desc = "Unknown",
            }
            table.add_row(row![addr_desc, format!("{addr:#X}")]);
            
            table.printstd();
        } else {
            table.set_titles(row!["Cannot parse TLP Format! "]);
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.add_row(row!["Tlp Format: ", tlpf]);
            table.printstd();
        }
	}

	fn display_cfg_req(&self, tlp: &TlpPacket) {
		let tlpf = tlp.get_tlp_format();
		let mut table = Table::new();

        if let Ok(tlpf1) = TlpFmt::try_from(tlpf) {
            if tlpf1 == TlpFmt::NoDataHeader4DW || tlpf1 == TlpFmt::WithDataHeader4DW {
                table.set_titles(row!["Configuration Requests are always 3DW! "]);
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.add_row(row!["Current request: ", tlpf1]);
                table.printstd();
                return;
            }

            let cfg = new_conf_req(tlp.get_data(), &tlpf1);
            let req_id = cfg.req_id();
            let tag = cfg.tag();
            let bus = cfg.bus_nr();
            let dev = cfg.dev_nr();
            let fun = cfg.func_nr();
            let ern = cfg.ext_reg_nr();
            let rn = cfg.reg_nr();

            table.set_titles(row!["TLP: ", tlp.get_tlp_format()]);
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.add_row(row!["Req ID", format!("{req_id:#X}")]);
            table.add_row(row!["Tag", format!("{tag:#X}")]);
            table.add_row(row!["Bus", format!("{bus:#X}")]);
            table.add_row(row!["Device", format!("{dev:#X}")]);
            table.add_row(row!["Function", format!("{fun:#X}")]);
            table.add_row(row!["Ext Reg Nr", format!("{ern:#X}")]);
            table.add_row(row!["Reg Nr", format!("{rn:#X}")]);
            
            table.printstd();
        } else {
            table.set_titles(row!["Cannot parse TLP Format! "]);
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.add_row(row!["Tlp Format: ", tlpf]);
            table.printstd();
        }
    }

	fn display_cmpl(&self, tlp: &TlpPacket) {
		let tlpf = tlp.get_tlp_format();
		let mut table = Table::new();

        if let Ok(tlpf1) = TlpFmt::try_from(tlpf) {
            if tlpf1 == TlpFmt::NoDataHeader4DW || tlpf1 == TlpFmt::WithDataHeader4DW {
                table.set_titles(row!["Completions are always 3DW! "]);
                table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
                table.add_row(row!["Current request: ", tlpf1]);
                table.printstd();
                return;
            }

            let cpl = new_cmpl_req(tlp.get_data(), &tlpf1);
            let cmp_id = cpl.cmpl_id();
            let cmp_st = cpl.cmpl_stat();
            let bcm = cpl.bcm();
            let bcnt = cpl.byte_cnt();
            let req_id = cpl.req_id();
            let tag  = cpl.tag();
            let laddr  = cpl.laddr();

            table.set_titles(row!["TLP: ", tlp.get_tlp_format()]);
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.add_row(row!["Compl ID", format!("{cmp_id:#X}")]);
            table.add_row(row!["Compl Status", format!("{cmp_st:#X}")]);
            table.add_row(row!["Byte Count Modified (PCI-X)", format!("{bcm:#X}")]);
            table.add_row(row!["Byte Count", format!("{bcnt:#X}")]);
            table.add_row(row!["Req ID", format!("{req_id:#X}")]);
            table.add_row(row!["Tag", format!("{tag:#X}")]);
            table.add_row(row!["Lower Address", format!("{laddr:#X}")]);
            
            table.printstd();
        } else {
            table.set_titles(row!["Cannot parse TLP Format! "]);
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.add_row(row!["Tlp Format: ", tlpf]);
            table.printstd();
        }

    }

	fn display_message_req(&self, tlp: &TlpPacket) {
        let tlpf = tlp.get_tlp_format();
        let mut table = Table::new();

        if let Ok(tlfp1) = TlpFmt::try_from(tlpf) {
            let msg = new_msg_req(tlp.get_data(), &tlfp1);
            let req_id = msg.req_id();
            let tag = msg.tag();
            let msg_code = msg.msg_code();
            let msg_dw3 = msg.dw3();
            let msg_dw4 = msg.dw4();
            
            table.set_titles(row!["TLP: ", tlp.get_tlp_format()]);
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.add_row(row!["Req ID", format!("{req_id:#X}")]);
            table.add_row(row!["Tag", format!("{tag:#X}")]);
            table.add_row(row!["Message Code", format!("{msg_code:#X}")]);
            table.add_row(row!["Message DW3", format!("{msg_dw3:#X}")]);
            table.add_row(row!["Message DW4", format!("{msg_dw4:#X}")]);

            table.printstd();
        }

    }
	fn display_uninplemented(&self, _tlp: &TlpPacket) { println!("Display not implemented yet"); }

	fn display_tlp_body(&self, tlp: &TlpPacket) {
		let tlpt = tlp.get_tlp_type();

		match tlpt {
			TlpType::MemReadReq				=> self.display_mem_req(tlp),
			TlpType::MemReadLockReq			=> self.display_mem_req(tlp),
			TlpType::MemWriteReq			=> self.display_mem_req(tlp),
			TlpType::IOReadReq				=> self.display_mem_req(tlp),
			TlpType::IOWriteReq				=> self.display_mem_req(tlp),
			TlpType::ConfType0ReadReq		=> self.display_cfg_req(tlp),
			TlpType::ConfType0WriteReq		=> self.display_cfg_req(tlp),
			TlpType::ConfType1ReadReq		=> self.display_cfg_req(tlp),
			TlpType::ConfType1WriteReq		=> self.display_cfg_req(tlp),
			TlpType::MsgReq					=> self.display_message_req(tlp),
			TlpType::MsgReqData				=> self.display_message_req(tlp),
			TlpType::Cpl					=> self.display_cmpl(tlp),
			TlpType::CplData				=> self.display_cmpl(tlp),
			TlpType::CplLocked				=> self.display_cmpl(tlp),
			TlpType::CplDataLocked			=> self.display_cmpl(tlp),
			TlpType::FetchAddAtomicOpReq	=> self.display_mem_req(tlp),
			TlpType::SwapAtomicOpReq		=> self.display_mem_req(tlp),
			TlpType::CompareSwapAtomicOpReq	=> self.display_mem_req(tlp),
			TlpType::LocalTlpPrefix			=> self.display_uninplemented(tlp),
			TlpType::EndToEndTlpPrefix		=> self.display_uninplemented(tlp),
		};
	}

    fn display_tlp_info(&self, tlp: &TlpPacket) {
		self.display_tlp_type(tlp);
        self.display_tlp_header(tlp.get_header());
		self.display_tlp_body(tlp);
    }

    fn run(&self) {
        let tlp = TlpPacket::new(self.config.input.to_vec());

        self.display_tlp_info(&tlp);
    }
}

enum ParseConfigError {
    InvalidInput,
}

impl Config {
    fn remove_whitespace(s: &str) -> String {
            s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    fn convert_to_vec(s: &str) -> Result<Vec<u8>, ()> {
        const RADIX: u32 = 16;
        let mut bytes: Vec<u8> = Vec::new();

		// Filter string and report error on invalid character
        for c in s.chars() {
            match c.to_digit(RADIX) {
                Some(d) => bytes.push(d as u8),
                None => return Result::Err(())
            }
        }

		// We already have valid u4 array, now need to convert to u8
		let mut result: Vec<u8> = Vec::new();
		let hex_u8: Vec<&[u8]> = bytes.chunks(2).collect();
		for h in hex_u8.iter() {
			result.push((h[0] << 4) + h[1]);
		}

        Result::Ok(result)
    }

    fn new(args: &Args) -> Result<Config, ParseConfigError> {

        let input = Config::remove_whitespace(&args.input);

        match Config::convert_to_vec(&input) {
            Ok(vec) => Result::Ok(Config {input: vec}),
            Err(()) => Result::Err(ParseConfigError::InvalidInput),
        }
    }
}

fn main() {
    let config = Config::new(&Args::parse());

    match config {
        Result::Ok(c) => {
            TlpTool::new(c).run();
        }
        Result::Err(e) => {
            match e {
                ParseConfigError::InvalidInput => println!("provided Input is invalid"),
            }
        }
    }
}

