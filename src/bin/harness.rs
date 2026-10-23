use m68k::AddressBus;
use opencdi::machine::Machine;
use opencdi::mcd212::{VideoMode, SCREEN_HEIGHT, SCREEN_WIDTH};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn video_hash(h: u64, fb: &[u32]) -> u64 {
    let mut h = h;
    for &p in fb {
        h = fnv(h, &p.to_le_bytes());
    }
    h
}

fn drain_audio(m: &mut Machine, buf: &mut Vec<(f32, f32)>, h: &mut u64, samples: &mut u64) {
    buf.clear();
    let rate = m.drain_audio(buf);
    for &(l, r) in buf.iter() {
        *h = fnv(*h, &l.to_bits().to_le_bytes());
        *h = fnv(*h, &r.to_bits().to_le_bytes());
        *samples += 1;
    }
    *h = fnv(*h, &rate.to_le_bytes());
}

fn dump_ppm(fb: &[u32], path: &str) {
    let mut v = format!("P6\n{} {}\n255\n", SCREEN_WIDTH, SCREEN_HEIGHT).into_bytes();
    for &p in fb {
        v.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
    }
    std::fs::write(path, v).expect("write ppm");
}

fn usage() -> ! {
    eprintln!("usage: harness <bios.rom> <disc|none> <frames> [--dvc <rom>] [--ppm <out.ppm>] [--ntsc] [--autoplay]");
    std::process::exit(2);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        usage();
    }
    let (bios, disc) = (&a[1], &a[2]);
    let frames: u64 = a[3].parse().unwrap_or_else(|_| usage());
    let mut dvc: Option<&String> = None;
    let mut ppm: Option<&String> = None;
    let mut ntsc = false;
    let mut autoplay = false;
    let mut i = 4;
    while i < a.len() {
        match a[i].as_str() {
            "--dvc" => { i += 1; dvc = a.get(i); }
            "--ppm" => { i += 1; ppm = a.get(i); }
            "--ntsc" => ntsc = true,
            "--autoplay" => autoplay = true,
            _ => usage(),
        }
        i += 1;
    }

    let mut m = Machine::new();
    m.load_bios(bios).expect("load bios");
    if let Some(p) = dvc {
        m.load_dvc_rom(p).expect("load dvc rom");
    }
    if disc != "none" {
        m.load_disc(disc).expect("load disc");
    }
    m.set_video_mode(if ntsc { VideoMode::Ntsc } else { VideoMode::Pal });
    m.reset();

    let mut video_all = FNV_OFFSET;
    let mut audio_all = FNV_OFFSET;
    let mut audio_samples = 0u64;
    let mut disc_launched = false;
    let mut shell_ready_count = 0u32;
    let mut audio_buf: Vec<(f32, f32)> = Vec::new();

    for frame in 0..frames {
        if autoplay && m.bus.cdrom.is_some() && !disc_launched {
            let (bus, cpu) = (&mut m.bus, &m.cpu);
            let shell_state = if bus.dvc.is_fitted() {
                let s = bus.read_long(0x00D08090u32.wrapping_add(0xFFFF9172));
                if s >= 2 { s } else { bus.read_long(0x0000C000u32.wrapping_add(0xFFFF9172)) }
            } else {
                bus.read_long(0x0000C000u32.wrapping_add(0xFFFF9172))
            };
            if shell_state >= 2 {
                shell_ready_count += 1;
                if shell_ready_count == 1 || shell_ready_count == 150 {
                    bus.slave.update_mouse(575, 335, 0);
                } else if shell_ready_count == 30 || shell_ready_count == 180 {
                    bus.slave.update_mouse(575, 335, 1);
                } else if shell_ready_count == 50 || shell_ready_count == 200 {
                    bus.slave.update_mouse(575, 335, 0);
                }
            }
            if ((cpu.pc >= 0x0001_0000 && cpu.pc < 0x0030_0000) || (cpu.pc >= 0x00D0_0000 && cpu.pc < 0x00E0_0000)) && frame > 250 {
                disc_launched = true;
                bus.slave.update_mouse(384, 280, 0);
            }
        }

        m.run_frame();

        video_all = video_hash(video_all, m.framebuffer());
        drain_audio(&mut m, &mut audio_buf, &mut audio_all, &mut audio_samples);
    }

    let video_last = video_hash(FNV_OFFSET, m.framebuffer());
    let nonblack = m.framebuffer().iter().filter(|&&p| p & 0x00FF_FFFF != 0).count();
    if let Some(p) = ppm {
        dump_ppm(m.framebuffer(), p);
    }
    println!(
        "\nHARNESS frames={} pc={:08x} nonblack={} video_last={:016x} video_all={:016x} audio={:016x} audio_samples={}",
        frames, m.cpu.pc, nonblack, video_last, video_all, audio_all, audio_samples
    );
}
