use m68k::{AddressBus, CpuCore, CpuType};
use opencdi::bus::CdiBus;
use opencdi::mcd212::{VideoMode, SCREEN_HEIGHT, SCREEN_WIDTH};
use std::sync::atomic::Ordering;

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

fn drain_audio(bus: &CdiBus, h: &mut u64, samples: &mut u64) {
    for q in [&bus.cdic.audio.sample_queue, &bus.cdic.audio.sfx_queue] {
        let mut g = q.lock().unwrap();
        for (l, r) in g.drain(..) {
            *h = fnv(*h, &l.to_bits().to_le_bytes());
            *h = fnv(*h, &r.to_bits().to_le_bytes());
            *samples += 1;
        }
    }
    let rate = bus.cdic.audio.source_rate.load(Ordering::Relaxed);
    *h = fnv(*h, &rate.to_le_bytes());
}

fn dump_ppm(bus: &CdiBus, path: &str) {
    let mut v = format!("P6\n{} {}\n255\n", SCREEN_WIDTH, SCREEN_HEIGHT).into_bytes();
    for &p in &bus.mcd212.framebuffer[..] {
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

    let mut bus = CdiBus::new();
    bus.load_rom(bios).expect("load bios");
    if let Some(p) = dvc {
        bus.load_dvc_rom(p).expect("load dvc rom");
    }
    if disc != "none" {
        bus.load_cdrom(disc).expect("load disc");
    }
    bus.set_video_mode(if ntsc { VideoMode::Ntsc } else { VideoMode::Pal });

    let mut cpu = CpuCore::new();
    cpu.set_cpu_type(CpuType::SCC68070);
    cpu.reset(&mut bus);
    bus.emulate_wait_states = true;

    let mut video_all = FNV_OFFSET;
    let mut audio_all = FNV_OFFSET;
    let mut audio_samples = 0u64;
    let mut disc_launched = false;
    let mut shell_ready_count = 0u32;

    for frame in 0..frames {
        if autoplay && bus.cdrom.is_some() && !disc_launched {
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

        bus.slave.video_mode = bus.mcd212.video_mode;
        let total_lines = bus.mcd212.video_mode.total_scanlines();
        let cyc_per_line = bus.mcd212.video_mode.cycles_per_scanline();
        let mut cycle_debt: i32 = 0;
        for line in 0..total_lines {
            bus.tick_scanline(line);

            if bus.mcd212.vblank_irq {
                bus.scc_timer.lir |= 0x80;
                bus.mcd212.vblank_irq = false;
            }
            cpu.set_irq(highest_ipl(&mut bus));

            let mut line_cycles = cycle_debt;
            while line_cycles < cyc_per_line as i32 {
                let remaining = (cyc_per_line as i32 - line_cycles).max(1);
                let budget = remaining.min(32);

                bus.current_pc = cpu.pc;
                bus.current_line_cycle = line_cycles.max(0) as u32;

                let batch = cpu.run_for_cycles(&mut bus, budget);
                let mut batch_cycles = batch.cycles + bus.consume_wait_cycles();

                match batch.exit {
                    m68k::CycleBatchExit::TrapInstruction { trap_num } => {
                        batch_cycles += cpu.take_trap_exception(&mut bus, trap_num);
                    }
                    m68k::CycleBatchExit::AlineTrap { .. } => {
                        batch_cycles += cpu.take_aline_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::FlineTrap { .. } => {
                        batch_cycles += cpu.take_fline_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::IllegalInstruction { .. } => {
                        batch_cycles += cpu.take_illegal_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::Breakpoint { .. } => {
                        batch_cycles += cpu.take_bkpt_exception(&mut bus);
                    }
                    m68k::CycleBatchExit::Stopped => {
                        let remaining_line = (cyc_per_line as i32 - line_cycles).max(0) as u32;
                        bus.tick_timer(remaining_line);
                        line_cycles = cyc_per_line as i32;
                        break;
                    }
                    _ => {}
                }

                line_cycles += batch_cycles;
                bus.tick_timer(batch_cycles as u32);
                cpu.set_irq(highest_ipl(&mut bus));
            }

            cycle_debt = (line_cycles - cyc_per_line as i32).clamp(0, 100);

            bus.render_scanline(line);

            bus.tick_slave(cyc_per_line);
            bus.tick_cdic(cyc_per_line);
            bus.tick_dvc(cyc_per_line);
        }

        video_all = video_hash(video_all, &bus.mcd212.framebuffer[..]);
        drain_audio(&bus, &mut audio_all, &mut audio_samples);
    }

    let video_last = video_hash(FNV_OFFSET, &bus.mcd212.framebuffer[..]);
    let nonblack = bus.mcd212.framebuffer.iter().filter(|&&p| p & 0x00FF_FFFF != 0).count();
    if let Some(p) = ppm {
        dump_ppm(&bus, p);
    }
    println!(
        "\nHARNESS frames={} pc={:08x} nonblack={} video_last={:016x} video_all={:016x} audio={:016x} audio_samples={}",
        frames, cpu.pc, nonblack, video_last, video_all, audio_all, audio_samples
    );
}

fn highest_ipl(bus: &mut CdiBus) -> u8 {
    let int1_lvl = if (bus.scc_timer.lir & 0x80) != 0 {
        let lvl = (bus.scc_timer.lir >> 4) & 7;
        if lvl != 0 { lvl } else { 5 }
    } else {
        0
    };
    let timer_lvl = if bus.scc_timer.timer_irq { bus.scc_timer.picr1 & 7 } else { 0 };
    let cdic_lvl = if bus.in4_pending() { 4 } else { 0 };
    let slave_lvl = if bus.slave.irq_pending { 2 } else { 0 };
    int1_lvl.max(timer_lvl).max(cdic_lvl).max(slave_lvl)
}
