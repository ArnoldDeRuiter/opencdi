use std::fs;

fn main() {
    let rom = fs::read("file utili/bios/cdi220b.rom")
        .or_else(|_| fs::read("../file utili/bios/cdi220b.rom"))
        .expect("cdi220b.rom not found");
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--find-target" {
        let target = u32::from_str_radix(args[2].trim_start_matches("0x").trim_start_matches("0X"), 16).unwrap();
        println!("Scanning for references to 0x{:06X}...", target);
        // 1. Literal 32-bit address
        let target_bytes = target.to_be_bytes();
        for i in 0..rom.len() - 4 {
            if rom[i..i+4] == target_bytes {
                println!("ROM[0x{:06X}] (PC 0x{:06X}): Literal address 0x{:08X}", i, 0x400000 + i as u32, target);
            }
        }
        // 2. PC-relative BSR / BRA / LEA / PEA
        for i in (0..rom.len() - 4).step_by(2) {
            let pc = 0x400000 + i as u32;
            let op = u16::from_be_bytes([rom[i], rom[i+1]]);
            // BSR.s or BRA.s
            if (op & 0xF000) == 0x6000 {
                let disp8 = (op & 0xFF) as i8 as i32;
                if disp8 != 0 && (pc as i32 + 2 + disp8) as u32 == target {
                    println!("0x{:06X}: Branch.S (disp8={}) -> 0x{:06X}", pc, disp8, target);
                } else if disp8 == 0 && i + 3 < rom.len() {
                    let disp16 = i16::from_be_bytes([rom[i+2], rom[i+3]]) as i32;
                    if (pc as i32 + 2 + disp16) as u32 == target {
                        println!("0x{:06X}: Branch.W (disp16={}) -> 0x{:06X}", pc, disp16, target);
                    }
                }
            }
            // LEA / PEA d16(PC)
            if (op & 0xF1C0) == 0x41FA || (op & 0xFFC0) == 0x487A {
                if i + 3 < rom.len() {
                    let disp16 = i16::from_be_bytes([rom[i+2], rom[i+3]]) as i32;
                    if (pc as i32 + 2 + disp16) as u32 == target {
                        println!("0x{:06X}: LEA/PEA d16(PC) (disp16={}) -> 0x{:06X}", pc, disp16, target);
                    }
                }
            }
        }
        return;
    }
    if args.len() >= 3 && args[1] == "--find-offset" {
        let target = u16::from_str_radix(args[2].trim_start_matches("0x").trim_start_matches("0X"), 16).unwrap();
        println!("Scanning for offset 0x{:04X}...", target);
        let target_bytes = target.to_be_bytes();
        for i in 0..rom.len() - 2 {
            if rom[i..i+2] == target_bytes {
                let pc = 0x400000 + i as u32;
                println!("Offset 0x{:04X} at PC=0x{:06X}", target, pc);
            }
        }
        return;
    }
    let ranges: Vec<(u32, u32, &str)> = if args.len() >= 3 {
        let start = u32::from_str_radix(args[1].trim_start_matches("0x").trim_start_matches("0X"), 16).unwrap();
        let end = u32::from_str_radix(args[2].trim_start_matches("0x").trim_start_matches("0X"), 16).unwrap();
        vec![(start, end, "Custom Range")]
    } else {
        vec![(0x465E80, 0x465EF0, "0x465E92 CD-i success branch")]
    };

    for (start, end, label) in ranges {
        println!("=== {} (0x{:06X} - 0x{:06X}) ===", label, start, end);
        let (rom_data, base_addr) = if start >= 0x07B000 && start < 0x080000 {
            (fs::read("cdapdriv.bin").expect("cdapdriv.bin not found"), 0x07B530u32)
        } else if start < 0x100000 {
            (fs::read("../cdi_bd13.bin").or_else(|_| fs::read("cdi_bd13.bin")).expect("cdi_bd13.bin not found"), 0x0u32)
        } else if start >= 0xDE0000 && start < 0xE00000 {
            (fs::read("../cdi_bd13.bin").or_else(|_| fs::read("cdi_bd13.bin")).expect("cdi_bd13.bin not found"), 0xDE09D0u32)
        } else if start >= 0xE00000 {
            (fs::read("file utili/dvc/vmpega.rom").or_else(|_| fs::read("../file utili/dvc/vmpega.rom")).expect("vmpega.rom not found"), 0xE40000u32)
        } else {
            (rom.clone(), 0x400000u32)
        };
        let mut pc = start;
        while pc < end {
            let offset = (pc - base_addr) as usize;
            if offset + 1 >= rom_data.len() { break; }
            let op = ((rom_data[offset] as u16) << 8) | (rom_data[offset + 1] as u16);
            let (dis, len) = m68k::dasm::disassemble(pc, op, m68k::CpuType::SCC68070);
            let mut bytes_str = String::new();
            for i in 0..len as usize {
                if offset + i < rom_data.len() {
                    bytes_str.push_str(&format!("{:02X} ", rom_data[offset + i]));
                }
            }
            println!("0x{:06X}  {:<16}  {}", pc, bytes_str, dis);
            pc += len.max(2) as u32;
        }
        println!();
    }
}
