use std::path::PathBuf;
use opencdi::cdrom::CdRom;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let game = if args.len() > 1 {
        args[1].clone()
    } else {
        "D:/Emulatori/Mame/Software/cdimono1/Space Ace (USA).chd".to_string()
    };

    println!("Loading CD-ROM: {}", game);
    let mut cd = CdRom::open(PathBuf::from(&game))?;
    println!("Total sectors (LBA count): {}", cd.total_sectors);
    println!("Disc type: 0x{:02X}", cd.disc_type);
    println!("Tracks count: {}", cd.tracks.len());
    for t in &cd.tracks {
        println!("  Track {}: control=0x{:02X}, start_lba={}, length={}, sector_size={}",
            t.number, t.control, t.start_lba, t.length_sectors, t.sector_size);
    }
    let out_name = "cdi_bd13.bin";
    let mut file_data = Vec::new();
    let mut sbuf = [0u8; 2352];
    for sec in 0..100 {
        if cd.read_sector(2273 + sec, &mut sbuf) {
            file_data.extend_from_slice(&sbuf[24..2072]);
        }
    }
    std::fs::write(out_name, &file_data)?;
    println!("Extracted {} ({} bytes)", out_name, file_data.len());

    let mut buf = [0u8; 2352];
    println!("Scanning sectors 0-4840 for 'bd13.rtf' and 'misc.rtf':");
    for sec in 0..4840 {
        if cd.read_sector(sec, &mut buf) {
            let data = &buf[24..2072];
            for i in 0..data.len().saturating_sub(8) {
                if &data[i..i+8] == b"bd13.rtf" || &data[i..i+8] == b"misc.rtf" {
                    let name = std::str::from_utf8(&data[i..i+8]).unwrap();
                    println!("Found {} in sector {} at offset {}", name, sec, i);
                    let start = i.saturating_sub(32);
                    let end = (i + 32).min(data.len());
                    println!("  bytes: {:02X?}", &data[start..end]);
                }
            }
        }
    }

    println!("Sectors 4875..4885 inspection:");
    for sec in 4875..4885 {
        cd.read_sector(sec, &mut buf);
        let submode = buf[18];
        let coding = buf[19];
        let file_num = buf[16];
        let chan_num = buf[17];
        println!("Sector {}: submode=0x{:02X} (data={}, audio={}, video={}, eor={}, eof={}) file={} chan={} coding=0x{:02X}",
            sec, submode,
            (submode & 0x08) != 0, (submode & 0x04) != 0, (submode & 0x02) != 0, (submode & 0x01) != 0, (submode & 0x80) != 0,
            file_num, chan_num, coding);
    }
    return Ok(());
}
