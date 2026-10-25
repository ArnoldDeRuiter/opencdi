#[path = "../cdrom.rs"]
mod cdrom;

use std::path::{Path, PathBuf};
use crate::cdrom::{CdRom, CD_SECTOR_RAW_SIZE};

fn find_file(relative_subpath: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new(relative_subpath).to_path_buf(),
        Path::new("..").join(relative_subpath),
        Path::new("file utili").join(relative_subpath),
        Path::new("..").join("file utili").join(relative_subpath),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

fn generate_sector_scramble() -> [u8; 2448] {
    let mut table = [0u8; 2448];
    let mut reg: u16 = 0x0001;
    for i in 12..2448 {
        let mut byte_val = 0u8;
        for bit in 0..8 {
            let out_bit = (reg & 1) as u8;
            byte_val |= out_bit << bit;
            let feedback = (reg ^ (reg >> 1)) & 1;
            reg = (reg >> 1) | (feedback << 14);
        }
        table[i] = byte_val;
    }
    table
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 4 && args[1] == "dump" {
        let out_file = &args[2];
        let start_lba: u32 = args[3].parse().unwrap();
        let end_lba: u32 = args[4].parse().unwrap();
        let game_path = if args.len() > 5 { PathBuf::from(&args[5]) } else { PathBuf::from("D:/Emulatori/Mame/Software/cdimono1/Brain Dead 13 (Europe).chd") };
        let mut cd = CdRom::open(&game_path)?;
        let mut sector_buf = [0u8; CD_SECTOR_RAW_SIZE];
        let mut out = Vec::new();
        for lba in start_lba..=end_lba {
            if cd.read_sector(lba, &mut sector_buf) {
                out.extend_from_slice(&sector_buf[24..24+2048]);
            }
        }
        std::fs::write(out_file, out)?;
        println!("Dumped {} sectors to {}", end_lba - start_lba + 1, out_file);
        return Ok(());
    }
    let game_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        find_file("giochi/Dragon's Lair (USA).chd").expect("Dragon's Lair CHD not found")
    };
    let start_lba: u32 = if args.len() > 2 { args[2].parse().unwrap_or(4080) } else { 4080 };
    let end_lba: u32 = if args.len() > 3 { args[3].parse().unwrap_or(4092) } else { start_lba + 15 };
    let mut cd = CdRom::open(&game_path)?;
    let mut sector_buf = [0u8; CD_SECTOR_RAW_SIZE];

    for lba in start_lba..=end_lba {
        if cd.read_sector(lba, &mut sector_buf) {
            let submode = sector_buf[18];
            let coding = sector_buf[19];
            let file = sector_buf[16];
            let chan = sector_buf[17];
            println!("LBA {}: file={}, chan={}, submode=0x{:02X}, coding=0x{:02X}, raw[16..24]:", lba, file, chan, submode, coding);
            for b in &sector_buf[16..24] { print!("{:02X} ", b); }
            println!();
            print!("   data[24..40]: ");
            for b in &sector_buf[24..40] { print!("{:02X} ", b); }
            println!();
        }
    }
    Ok(())
}
