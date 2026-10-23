// OpenCDi Release Build - 2026-09-29 15:45
use std::path::{Path, PathBuf};
use log::{info, error};
use minifb::{Key, Window, WindowOptions};

use opencdi::audio::HostAudioOutput;
use opencdi::frontend::{pop_action, push_action, GuiAction, InputDeviceMode, KeyBindings};
#[cfg(target_os = "windows")]
use opencdi::gui;
use opencdi::machine::Machine;
use opencdi::mcd212::{SCREEN_WIDTH, SCREEN_HEIGHT, VideoMode};

#[cfg(target_os = "windows")]
#[repr(C)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[link(name = "winmm")]
unsafe extern "system" {
    fn ShowWindow(hwnd: *mut std::ffi::c_void, n_cmd_show: i32) -> i32;
    fn IsZoomed(hwnd: *mut std::ffi::c_void) -> i32;
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn ShowCursor(b_show: i32) -> i32;
    fn ClipCursor(lp_rect: *const RECT) -> i32;
    fn GetClientRect(hwnd: *mut std::ffi::c_void, lp_rect: *mut RECT) -> i32;
    fn ClientToScreen(hwnd: *mut std::ffi::c_void, lp_point: *mut POINT) -> i32;
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, lptext: *const u16, lpcaption: *const u16, utype: u32) -> i32;
}

#[cfg(target_os = "windows")]
fn set_cursor_visibility(visible: bool, is_currently_hidden: &mut bool) {
    unsafe {
        if !visible && !*is_currently_hidden {
            while ShowCursor(0) >= 0 {}
            *is_currently_hidden = true;
        } else if visible && *is_currently_hidden {
            while ShowCursor(1) < 0 {}
            *is_currently_hidden = false;
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn set_cursor_visibility(_visible: bool, _is_currently_hidden: &mut bool) {}

#[cfg(target_os = "windows")]
fn set_mouse_confinement(confined: bool, hwnd: *mut std::ffi::c_void, is_currently_clipped: &mut bool) {
    unsafe {
        if confined && !hwnd.is_null() {
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rect);
            let mut tl = POINT { x: rect.left, y: rect.top };
            let mut br = POINT { x: rect.right, y: rect.bottom };
            ClientToScreen(hwnd, &mut tl);
            ClientToScreen(hwnd, &mut br);
            let screen_rect = RECT {
                left: tl.x,
                top: tl.y,
                right: br.x,
                bottom: br.y,
            };
            ClipCursor(&screen_rect);
            *is_currently_clipped = true;
        } else if !confined && *is_currently_clipped {
            ClipCursor(std::ptr::null());
            *is_currently_clipped = false;
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn set_mouse_confinement(_confined: bool, _hwnd: *mut std::ffi::c_void, _is_currently_clipped: &mut bool) {}


fn find_file(relative_subpath: &str) -> Option<PathBuf> {
    let sub = Path::new(relative_subpath);
    let leaf_name = sub.file_name().and_then(|s| s.to_str()).unwrap_or(relative_subpath);

    let mut candidate_bases: Vec<PathBuf> = vec![
        PathBuf::from("."),
        PathBuf::from(".."),
        PathBuf::from("file utili"),
        PathBuf::from("..").join("file utili"),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidate_bases.push(exe_dir.to_path_buf());
            candidate_bases.push(exe_dir.join(".."));
            candidate_bases.push(exe_dir.join("file utili"));
            candidate_bases.push(exe_dir.join("..").join("file utili"));
        }
    }

    for base in &candidate_bases {
        // 1. Try exact relative subpath (e.g., bios/cdi220b.rom)
        let exact = base.join(relative_subpath);
        if exact.is_file() {
            return Some(exact);
        }
        // 2. Try direct leaf filename (e.g., cdi220b.rom alongside executable)
        let leaf = base.join(leaf_name);
        if leaf.is_file() {
            return Some(leaf);
        }
    }
    None
}

fn find_bios_rom() -> Option<PathBuf> {
    const BIOS_CANDIDATES: &[&str] = &[
        "bios/cdi220b.rom",
        "cdi220b.rom",
        "bios/cdi220.rom",
        "cdi220.rom",
        "bios/cdi200.rom",
        "cdi200.rom",
        "bios/cdi.rom",
        "cdi.rom",
        "bios/cdi490a.rom",
        "cdi490a.rom",
    ];
    for &name in BIOS_CANDIDATES {
        if let Some(p) = find_file(name) {
            return Some(p);
        }
    }
    None
}

fn find_dvc_rom() -> Option<PathBuf> {
    const DVC_CANDIDATES: &[&str] = &[
        "dvc/vmpega.rom",
        "bios/vmpega.rom",
        "vmpega.rom",
        "dvc/impega.rom",
        "bios/impega.rom",
        "impega.rom",
    ];
    for &name in DVC_CANDIDATES {
        if let Some(p) = find_file(name) {
            return Some(p);
        }
    }
    None
}

fn find_nvram_file() -> Option<PathBuf> {
    const NVRAM_CANDIDATES: &[&str] = &[
        "bios/nvram_mk48t08.bin",
        "nvram_mk48t08.bin",
        "bios/nvram.bin",
        "nvram.bin",
    ];
    for &name in NVRAM_CANDIDATES {
        if let Some(p) = find_file(name) {
            return Some(p);
        }
    }
    None
}

fn toggle_fullscreen(window: &Window) {
    #[cfg(target_os = "windows")]
    {
        let hwnd = window.get_window_handle();
        if !hwnd.is_null() {
            let is_max = unsafe { IsZoomed(hwnd) } != 0;
            unsafe {
                ShowWindow(hwnd, if is_max { 9 } else { 3 });
            }
            info!("Schermo intero: {}", if is_max { "FINESTRA" } else { "SCHERMO INTERO (4:3)" });
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        info!("Fullscreen toggle is Windows-only; maximise or resize the window instead");
    }
}

fn save_screenshot(frame_count: u64, framebuffer: &[u32]) {
    let _ = std::fs::create_dir_all("screenshots_hotel_mario");
    let filename = format!("screenshots_hotel_mario/screenshot_frame_{}.bmp", frame_count);
    let width = SCREEN_WIDTH as i32;
    let height = SCREEN_HEIGHT as i32;
    let file_size = 54 + (width * height * 4) as u32;
    let mut bmp = Vec::with_capacity(file_size as usize);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&(-height).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&((width * height * 4) as u32).to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    for &pixel in framebuffer {
        bmp.push((pixel & 0xFF) as u8);
        bmp.push(((pixel >> 8) & 0xFF) as u8);
        bmp.push(((pixel >> 16) & 0xFF) as u8);
        bmp.push(0xFF);
    }
    if let Ok(_) = std::fs::write(&filename, &bmp) {
        info!("Screenshot salvato in {}", filename);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    #[cfg(target_os = "windows")]
    unsafe {
        timeBeginPeriod(1);
    }

    info!("=== OpenCDi - Philips CD-i Emulator ===");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let has_flag = |f: &str| args.iter().any(|a| a == f);
    let flag_value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).cloned();
    let rom_arg = flag_value("--rom").map(PathBuf::from);
    let max_frames: Option<u64> = flag_value("--frames").and_then(|s| s.parse().ok());

    let mut m = Machine::new();
    m.bus.slave.debug_log = has_flag("--slave-debug");
    if m.bus.slave.debug_log {
        info!("Debug Slave XBUS abilitato (--slave-debug): log byte CH0 (mouse)");
    }

    // 1. Load BIOS ROM
    let bios_path = match rom_arg.or_else(find_bios_rom) {
        Some(path) => path,
        None => {
            let error_msg =
                "OpenCDi - Required System BIOS Missing\n\
                ======================================\n\n\
                The emulator cannot start because no Philips CD-i system BIOS ROM was found.\n\n\
                Recommended File:\n\
                  • cdi220b.rom (512 KB, Philips CD-i 220 European PAL BIOS)\n\
                    SHA-1: 53360A1F21DDAC952E95306CED64186A3FC0B93E\n\
                    MD5:   AC0D468BE366779C9DF633BE98DA250A\n\n\
                Alternative Supported BIOSes:\n\
                  • cdi220.rom, cdi200.rom, cdi490a.rom\n\n\
                Where to place the BIOS ROM:\n\
                  1. In a 'bios/' subfolder next to opencdi.exe\n\
                  2. OR directly in the same folder as opencdi.exe\n\
                  3. OR pass it on the command line: --rom <path>\n\n\
                Digital Video Cartridge (DVC) for MPEG / FMV games:\n\
                  • Place 'vmpega.rom' (256 KB) in 'dvc/', 'bios/', or next to opencdi.exe.";

            error!("{}", error_msg);

            #[cfg(target_os = "windows")]
            unsafe {
                use std::os::windows::ffi::OsStrExt;
                let wide_text: Vec<u16> = std::ffi::OsStr::new(error_msg)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let wide_title: Vec<u16> = std::ffi::OsStr::new("OpenCDi - Required BIOS Missing")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                MessageBoxW(
                    std::ptr::null_mut(),
                    wide_text.as_ptr(),
                    wide_title.as_ptr(),
                    0x0000_0000 | 0x0000_0010, // MB_OK | MB_ICONERROR
                );
            }
            std::process::exit(1);
        }
    };
    m.bus.load_rom(&bios_path)?;
    info!("Loaded system BIOS from {:?}", bios_path);

    // 2. Load NVRAM if present
    if let Some(nvram_path) = find_nvram_file() {
        let _ = m.bus.nvram.load_from_file(&nvram_path);
        info!("Loaded NVRAM from {:?}", nvram_path);
    }

    // 2b. Load DVC ROM (Digital Video Cartridge) by default if present
    let dvc_disabled = has_flag("--no-dvc") || has_flag("--nodvc");
    if !dvc_disabled {
        if let Some(dvc_path) = find_dvc_rom() {
            if let Ok(_) = m.bus.load_dvc_rom(&dvc_path) {
                info!("Loaded Digital Video Cartridge (DVC) ROM from {:?}", dvc_path);
            }
        } else {
            info!("No DVC ROM (vmpega.rom) found: booting in Base CD-i mode (MPEG video disabled).");
        }
    } else {
        info!("Forced boot without DVC (--no-dvc): Base CD-i mode");
    }

    // 3. Optional Game Disc (from command line argument, or None for standalone BIOS)
    let mut current_game_name = "No disc inserted (Standalone BIOS)".to_string();
    // Disc = first positional argument (skipping flag values); --bios / --standalone force no disc
    let standalone = has_flag("--bios") || has_flag("--standalone");
    let mut disc_arg = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--rom" || a == "--frames" {
            it.next();
        } else if !a.starts_with("--") {
            disc_arg = Some(a.clone());
            break;
        }
    }
    let initial_disc = disc_arg.filter(|_| !standalone).and_then(|a| {
        find_file(&a).or_else(|| {
            let p = PathBuf::from(&a);
            if p.exists() { Some(p) } else { None }
        })
    });

    if let Some(p) = initial_disc {
        match m.bus.load_cdrom(&p) {
            Ok(_) => {
                current_game_name = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "CD-i Disc".to_string());
                info!("Loaded game disc at startup: {}", current_game_name);

                let name_upper = current_game_name.to_uppercase();
                let is_ntsc = (name_upper.contains("(USA)") || name_upper.contains("(JAPAN)") || name_upper.contains("NTSC"))
                    && m.bus.cdrom.as_ref().map_or(true, |cd| cd.disc_type != 0x20);
                if is_ntsc {
                    info!("Detected NTSC disc (USA/Japan): auto-configured NTSC 60 Hz video mode");
                    m.bus.set_video_mode(VideoMode::Ntsc);
                } else {
                    info!("Console booted in standard PAL 50 Hz mode (CD-i 220 BIOS)");
                    m.bus.set_video_mode(VideoMode::Pal);
                }
            }
            Err(e) => error!("Error loading game disc from {:?}: {:?}", p, e),
        }
    } else {
        info!("Starting console in Standalone BIOS mode (no disc inserted) - PAL 50 Hz");
        m.bus.set_video_mode(VideoMode::Pal);
    }

    // 4. Initialize 68070 CPU Core
    m.reset();
    info!("CPU SCC68070 reset - PC: 0x{:08X}", m.cpu.pc);

    // 5. Create Display Window
    let mut window = Window::new(
        "OpenCDi - Philips CD-i",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: true,
            scale: minifb::Scale::X1,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )?;

    window.limit_update_rate(None);

    // 6. Initialize Controls Configuration
    let mut bindings = KeyBindings::new();
    let mut input_device_mode = if has_flag("--mouse") { InputDeviceMode::Mouse } else { InputDeviceMode::Gamepad };

    // 7. Attach Native Win32 Menu GUI
    #[cfg(target_os = "windows")]
    {
        let hwnd = window.get_window_handle();
        gui::attach_gui_menu(hwnd, bindings.profile, input_device_mode, m.bus.mcd212.video_mode);
        gui::update_dvc_menu(m.bus.dvc.is_fitted());
    }

    // 8. Initialize Host Audio Playback
    let _host_audio = HostAudioOutput::try_start(
        m.bus.cdic.audio.sample_queue.clone(),
        m.bus.cdic.audio.sfx_queue.clone(),
        m.bus.cdic.audio.source_rate.clone(),
    );

    // 9. Initialize USB Gamepad subsystem (Gilrs / XInput / DirectInput)
    let mut gilrs = gilrs::Gilrs::new().ok();
    if gilrs.is_some() {
        info!("USB Gamepad subsystem active (Xbox / PlayStation / Generic USB)");
    }

    let mut frame_count: u64 = 0;
    let mut frames_run: u64 = 0;
    let mut disc_launched = false;
    let mut shell_ready_count: u32 = 0;

    let mut pad_x: i32 = if m.bus.cdrom.is_some() { 575 } else { 384 };
    let mut pad_y: i32 = if m.bus.cdrom.is_some() { 335 } else { 280 };
    let mut pad_x_f: f32 = pad_x as f32;
    let mut pad_y_f: f32 = pad_y as f32;
    let mut pad_active = false;
    m.bus.emulate_wait_states = true; // Authentic hardware wait-states (15.5 MHz)
    let mut mouse_captured = true;
    let mut host_cursor_hidden = false;
    let mut mouse_clipped = false;
    let mut prev_mx: f32 = -1.0;
    let mut prev_my: f32 = -1.0;
    let mut prev_middle = false;
    let mut prev_f11 = false;
    let mut prev_s = false;
    let mut prev_m = false;
    let mut prev_ctrl_o = false;
    let mut prev_ctrl_i = false;
    let mut prev_f5 = false;
    let mut prev_key_btn: u8 = 0;

    info!("Emulation loop started. Use menu or shortcuts: [Ctrl+I] Insert Disc, [Ctrl+O] Open & Reset, [F5] Reset, [F11] Fullscreen, [Middle Click] Capture/Release Mouse");

    let mut fps_timer = std::time::Instant::now();
    let mut measured_fps = 50.0f64;
    let mut next_frame_time = std::time::Instant::now();

    let mut prev_instructions: u64 = 0;
    let mut prev_cd_sectors: u32 = 0;
    let mut prev_vsr = (m.bus.mcd212.get_vsr::<0>(), m.bus.mcd212.get_vsr::<1>());
    let mut buffer_flips_interval: u32 = 0;
    let mut measured_render_fps: f64 = 0.0;
    let mut measured_mips: f64 = 0.0;
    let mut measured_cd_rate: f64 = 0.0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Handle GUI Actions from Native Win32 Menu
        let mut exit_requested = false;
        while let Some(action) = pop_action() {
            match action {
                GuiAction::InsertDisc(path) => {
                    info!("Hot-swapping disc into CD-i drive: {:?}", path);
                    match m.bus.change_disc(&path) {
                        Ok(_) => {
                            current_game_name = path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "CD-i Disc".to_string());
                            info!("New disc inserted successfully: {}", current_game_name);
                        }
                        Err(e) => {
                            error!("Failed to hot-swap disc {:?}: {:?}", path, e);
                        }
                    }
                }
                GuiAction::OpenDisc(path) => {
                    info!("Loading and booting new disc (Reset): {:?}", path);
                    match m.bus.load_cdrom(&path) {
                        Ok(_) => {
                            current_game_name = path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "CD-i Disc".to_string());
                            info!("Disc loaded successfully: {}", current_game_name);

                            if !dvc_disabled && !m.bus.dvc.is_fitted() {
                                if let Some(dvc_path) = find_dvc_rom() {
                                    if let Ok(_) = m.bus.load_dvc_rom(&dvc_path) {
                                        info!("Loaded Digital Video Cartridge (DVC) extension from {:?}", dvc_path);
                                        #[cfg(target_os = "windows")]
                                        gui::update_dvc_menu(true);
                                    }
                                }
                            }

                            let name_upper = current_game_name.to_uppercase();
                            let is_ntsc = (name_upper.contains("(USA)") || name_upper.contains("(JAPAN)") || name_upper.contains("NTSC"))
                                && m.bus.cdrom.as_ref().map_or(true, |cd| cd.disc_type != 0x20);
                            let mode = if is_ntsc {
                                info!("Detected NTSC disc (USA/Japan): auto-configured NTSC 60 Hz video mode");
                                VideoMode::Ntsc
                            } else {
                                VideoMode::Pal
                            };
                            m.bus.set_video_mode(mode);
                            #[cfg(target_os = "windows")]
                            gui::update_video_menu(mode);

                            m.bus.reset_system();
                            m.cpu.reset(&mut m.bus);
                            frame_count = 0;
                            disc_launched = false;
                            shell_ready_count = 0;
                            pad_x = 575;
                            pad_y = 335;
                            pad_x_f = 575.0;
                            pad_y_f = 335.0;
                            pad_active = false;
                            mouse_captured = true;
                            m.bus.emulate_wait_states = true;
                            next_frame_time = std::time::Instant::now();
                            fps_timer = std::time::Instant::now();
                        }
                        Err(e) => {
                            error!("Failed to load disc {:?}: {:?}", path, e);
                        }
                    }
                }
                GuiAction::EjectDisc => {
                    info!("Ejecting disc from CD-i drive");
                    m.bus.slave.set_disc_inserted(false);
                    m.bus.cdrom = None;
                    disc_launched = false;
                    shell_ready_count = 0;
                    current_game_name = "No disc inserted (Standalone BIOS)".to_string();
                    pad_x = 384;
                    pad_y = 280;
                    pad_x_f = 384.0;
                    pad_y_f = 280.0;
                }
                GuiAction::Reset => {
                    info!("Console reset requested");
                    m.bus.reset_system();
                    m.cpu.reset(&mut m.bus);
                    frame_count = 0;
                    disc_launched = false;
                    shell_ready_count = 0;
                    pad_x = if m.bus.cdrom.is_some() { 575 } else { 384 };
                    pad_y = if m.bus.cdrom.is_some() { 335 } else { 280 };
                    pad_x_f = pad_x as f32;
                    pad_y_f = pad_y as f32;
                    pad_active = false;
                    mouse_captured = true;
                    m.bus.emulate_wait_states = true;
                    next_frame_time = std::time::Instant::now();
                    fps_timer = std::time::Instant::now();
                }
                GuiAction::Exit => {
                    info!("Exit requested from File menu");
                    exit_requested = true;
                    break;
                }
                GuiAction::SetProfile(profile) => {
                    bindings.set_profile(profile);
                    #[cfg(target_os = "windows")]
                    gui::update_profile_menu(bindings.profile);
                    info!("Control profile updated to: {:?}", bindings.profile);
                }
                GuiAction::SetInputDevice(mode) => {
                    input_device_mode = mode;
                    #[cfg(target_os = "windows")]
                    gui::update_input_device_menu(mode);
                    info!("Input device mode set to: {:?}", mode);
                }
                GuiAction::ReloadControls => {
                    bindings.load_from_ini();
                    #[cfg(target_os = "windows")]
                    gui::update_profile_menu(bindings.profile);
                    info!("Controls reloaded from INI file. Profile: {:?}", bindings.profile);
                }
                GuiAction::ToggleReleaseMouse => {
                    mouse_captured = !mouse_captured;
                    info!("Mouse capture: {}", if mouse_captured { "CAPTURED" } else { "RELEASED" });
                }
                GuiAction::ToggleMute => {
                    m.bus.cdic.audio.muted = !m.bus.cdic.audio.muted;
                    info!("Audio: {}", if m.bus.cdic.audio.muted { "MUTED" } else { "UNMUTED" });
                }
                GuiAction::VolumeUp => {
                    m.bus.cdic.audio.volume = (m.bus.cdic.audio.volume + 0.05).min(2.0);
                }
                GuiAction::VolumeDown => {
                    m.bus.cdic.audio.volume = (m.bus.cdic.audio.volume - 0.05).max(0.0);
                }
                GuiAction::ToggleFullscreen => {
                    toggle_fullscreen(&window);
                }
                GuiAction::TakeScreenshot => {
                    save_screenshot(frame_count, &m.bus.mcd212.framebuffer[..]);
                }
                GuiAction::SetVideoMode(mode) => {
                    m.bus.set_video_mode(mode);
                    #[cfg(target_os = "windows")]
                    gui::update_video_menu(mode);
                    info!("Video mode set to: {:?}", mode);
                }
                GuiAction::ToggleWaitStates => {
                    m.bus.emulate_wait_states = !m.bus.emulate_wait_states;
                    #[cfg(target_os = "windows")]
                    gui::update_waitstates_menu(m.bus.emulate_wait_states);
                    info!("Authentic Hardware Speed (Wait-States): {}", if m.bus.emulate_wait_states { "ENABLED (Authentic CD-i 220)" } else { "DISABLED (Turbo Zero-Wait)" });
                }
                GuiAction::ToggleDvc => {
                    if m.bus.dvc.is_fitted() {
                        info!("Removing Digital Video Cartridge (Base CD-i mode)");
                        m.bus.dvc = opencdi::dvc::Dvc::new();
                        #[cfg(target_os = "windows")]
                        gui::update_dvc_menu(false);
                    } else {
                        if let Some(dvc_path) = find_dvc_rom() {
                            if let Ok(_) = m.bus.load_dvc_rom(&dvc_path) {
                                info!("Inserted Digital Video Cartridge (DVC) from {:?}", dvc_path);
                                #[cfg(target_os = "windows")]
                                gui::update_dvc_menu(true);
                            }
                        } else {
                            error!("Cannot enable DVC: vmpega.rom not found in dvc/, bios/, or root directory");
                        }
                    }
                    m.bus.reset_system();
                    m.cpu.reset(&mut m.bus);
                    frame_count = 0;
                    disc_launched = false;
                    shell_ready_count = 0;
                    pad_x = if m.bus.cdrom.is_some() { 575 } else { 384 };
                    pad_y = if m.bus.cdrom.is_some() { 335 } else { 280 };
                    pad_x_f = pad_x as f32;
                    pad_y_f = pad_y as f32;
                    pad_active = false;
                }
                GuiAction::ShowHelp => {
                    #[cfg(target_os = "windows")]
                    gui::show_controls_dialog(window.get_window_handle());
                }
                GuiAction::ShowAbout => {
                    #[cfg(target_os = "windows")]
                    gui::show_about_dialog(window.get_window_handle());
                }
            }
        }

        if exit_requested {
            break;
        }

        // Se un disco è presente, avvia automaticamente la riproduzione una sola volta quando la shell del BIOS è pronta
        if m.bus.cdrom.is_some() && !disc_launched {
            let (_effective_shell_base, shell_state) = if m.bus.dvc.is_fitted() {
                let s_dvc = m.bus.peek_long(0x00D08090u32.wrapping_add(0xFFFF9172)).unwrap_or(0);
                if s_dvc >= 2 {
                    (0x00D08090u32, s_dvc)
                } else {
                    (0x0000C000u32, m.bus.peek_long(0x0000C000u32.wrapping_add(0xFFFF9172)).unwrap_or(0))
                }
            } else {
                (0x0000C000u32, m.bus.peek_long(0x0000C000u32.wrapping_add(0xFFFF9172)).unwrap_or(0))
            };

            if shell_state >= 2 {
                shell_ready_count += 1;
                if shell_ready_count == 1 || shell_ready_count == 150 {
                    m.bus.slave.update_mouse(575, 335, 0);
                } else if shell_ready_count == 30 || shell_ready_count == 180 {
                    info!("Shell BIOS pronta (stato {}): avvio automatico disco CD-i (click PLAY CD-I)...", shell_state);
                    m.bus.slave.update_mouse(575, 335, 1);
                } else if shell_ready_count == 50 || shell_ready_count == 200 {
                    m.bus.slave.update_mouse(575, 335, 0);
                }
            }
            if ((m.cpu.pc >= 0x00010000 && m.cpu.pc < 0x00300000) || (m.cpu.pc >= 0x00D00000 && m.cpu.pc < 0x00E00000)) && frame_count > 250 {
                info!("Applicazione disco in esecuzione in RAM (PC=0x{:06X}), controllo passato all'utente.", m.cpu.pc);
                disc_launched = true;
                m.bus.emulate_wait_states = true;
                pad_x = 384;
                pad_y = 280;
                pad_x_f = 384.0;
                pad_y_f = 280.0;
                m.bus.slave.update_mouse(384, 280, 0);
            }
        }

        m.run_frame();

        let curr_vsr = (m.bus.mcd212.get_vsr::<0>(), m.bus.mcd212.get_vsr::<1>());
        if curr_vsr != prev_vsr {
            prev_vsr = curr_vsr;
            buffer_flips_interval += 1;
        }

        let is_active = window.is_active();
        let hwnd = window.get_window_handle();

        // Host cursor hiding and confinement
        if is_active && mouse_captured {
            set_cursor_visibility(false, &mut host_cursor_hidden);
            set_mouse_confinement(true, hwnd, &mut mouse_clipped);
        } else {
            set_cursor_visibility(true, &mut host_cursor_hidden);
            set_mouse_confinement(false, hwnd, &mut mouse_clipped);
        }

        // Keyboard Shortcut: Ctrl+I (Inserisci Disco a caldo)
        let ctrl_down = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
        let i_down = window.is_key_down(Key::I);
        if is_active && ctrl_down && i_down && !prev_ctrl_i {
            #[cfg(target_os = "windows")]
            {
                if let Some(path) = gui::open_file_dialog(hwnd) {
                    push_action(GuiAction::InsertDisc(path));
                }
            }
            #[cfg(not(target_os = "windows"))]
            info!("No file dialog on this platform: pass the disc path on the command line");
        }
        prev_ctrl_i = ctrl_down && i_down;

        // Keyboard Shortcut: Ctrl+O (Apri Gioco e Riavvia)
        let o_down = window.is_key_down(Key::O);
        if is_active && ctrl_down && o_down && !prev_ctrl_o {
            #[cfg(target_os = "windows")]
            {
                if let Some(path) = gui::open_file_dialog(hwnd) {
                    push_action(GuiAction::OpenDisc(path));
                }
            }
            #[cfg(not(target_os = "windows"))]
            info!("No file dialog on this platform: pass the disc path on the command line");
        }
        prev_ctrl_o = ctrl_down && o_down;

        // Keyboard Shortcut: F5 (Reset)
        let f5_down = window.is_key_down(Key::F5);
        if is_active && f5_down && !prev_f5 {
            push_action(GuiAction::Reset);
        }
        prev_f5 = f5_down;

        // F11 Fullscreen toggle
        let f11_down = window.is_key_down(Key::F11);
        if is_active && f11_down && !prev_f11 {
            toggle_fullscreen(&window);
        }
        prev_f11 = f11_down;

        // Middle mouse button (rotellina/tasto centrale): toggle mouse capture / release focus
        let middle_down = window.get_mouse_down(minifb::MouseButton::Middle);
        if is_active && middle_down && !prev_middle {
            mouse_captured = !mouse_captured;
            info!("Controllo mouse: {}", if mouse_captured { "CATTURATO (focus console, puntatore host nascosto)" } else { "LIBERO (puntatore desktop visibile, rotellina o clic per ricatturare)" });
        }
        prev_middle = middle_down;

        // Left click inside window re-captures mouse if released
        if is_active && !mouse_captured && window.get_mouse_down(minifb::MouseButton::Left) {
            mouse_captured = true;
            info!("Controllo mouse: CATTURATO (focus console, puntatore host nascosto)");
        }

        // Directional state from Keyboard (Arrow keys, WASD, or current profile)
        let mut dir_x: i32 = 0;
        let mut dir_y: i32 = 0;
        let mut user_input_active = false;

        if is_active {
            if window.is_key_down(bindings.left) || window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
                dir_x -= 1;
                user_input_active = true;
            }
            if window.is_key_down(bindings.right) || window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
                dir_x += 1;
                user_input_active = true;
            }
            if window.is_key_down(bindings.up) || window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
                dir_y -= 1;
                user_input_active = true;
            }
            if window.is_key_down(bindings.down) || window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
                dir_y += 1;
                user_input_active = true;
            }
        }

        // Button 1 (Action / Sword / Start) and Button 2 (Jump / Secondary)
        let mut key_btn = 0;
        if is_active {
            if window.is_key_down(bindings.btn1)
                || window.is_key_down(Key::Space)
                || window.is_key_down(Key::Enter)
                || window.is_key_down(Key::Z)
                || window.is_key_down(Key::J)
            {
                key_btn |= 1;
                user_input_active = true;
            }
            if window.is_key_down(bindings.btn2)
                || window.is_key_down(Key::LeftShift)
                || window.is_key_down(Key::RightShift)
                || window.is_key_down(Key::X)
                || window.is_key_down(Key::K)
            {
                key_btn |= 2;
                user_input_active = true;
            }
        }

        // Gamepad handling via Gilrs (Xbox / PlayStation / USB)
        if let Some(ref mut g) = gilrs {
            while let Some(_) = g.next_event() {} // Drain events to refresh state
            for (_id, gamepad) in g.gamepads() {
                if gamepad.is_connected() {
                    let mut pad_dir_x = 0;
                    let mut pad_dir_y = 0;

                    // 1. D-Pad buttons (highest priority)
                    if gamepad.is_pressed(gilrs::Button::DPadUp) { pad_dir_y = -1; }
                    else if gamepad.is_pressed(gilrs::Button::DPadDown) { pad_dir_y = 1; }

                    if gamepad.is_pressed(gilrs::Button::DPadLeft) { pad_dir_x = -1; }
                    else if gamepad.is_pressed(gilrs::Button::DPadRight) { pad_dir_x = 1; }

                    // 2. D-Pad axes (for gamepads where D-Pad is reported as Hat/Axis, only if button not pressed)
                    if pad_dir_x == 0 {
                        let dpx = gamepad.value(gilrs::Axis::DPadX);
                        if dpx < -0.4 { pad_dir_x = -1; }
                        else if dpx > 0.4 { pad_dir_x = 1; }
                    }
                    if pad_dir_y == 0 {
                        let dpy = gamepad.value(gilrs::Axis::DPadY);
                        if dpy > 0.4 { pad_dir_y = -1; }
                        else if dpy < -0.4 { pad_dir_y = 1; }
                    }

                    // 3. Left Analog Stick (with deadzone 0.35, only if D-pad is neutral on that axis)
                    if pad_dir_x == 0 {
                        let lx = gamepad.value(gilrs::Axis::LeftStickX);
                        if lx < -0.35 { pad_dir_x = -1; }
                        else if lx > 0.35 { pad_dir_x = 1; }
                    }
                    if pad_dir_y == 0 {
                        let ly = gamepad.value(gilrs::Axis::LeftStickY);
                        if ly > 0.35 { pad_dir_y = -1; }
                        else if ly < -0.35 { pad_dir_y = 1; }
                    }

                    if pad_dir_x != 0 {
                        dir_x += pad_dir_x;
                        user_input_active = true;
                    }
                    if pad_dir_y != 0 {
                        dir_y += pad_dir_y;
                        user_input_active = true;
                    }

                    // Button 1: South (A / Cross)
                    if gamepad.is_pressed(gilrs::Button::South) {
                        key_btn |= 1;
                        user_input_active = true;
                    }
                    // Button 2: East (B / Circle) or West (X / Square) -> Jump
                    if gamepad.is_pressed(gilrs::Button::East) || gamepad.is_pressed(gilrs::Button::West) {
                        key_btn |= 2;
                        user_input_active = true;
                    }
                    // Start Button -> Confirm / Action
                    if gamepad.is_pressed(gilrs::Button::Start) {
                        key_btn |= 1;
                        user_input_active = true;
                    }
                }
            }
        }

        // Mouse pointer handling: detect physical mouse movement or click
        let mut mouse_btn = 0;
        let mut mouse_x = pad_x;
        let mut mouse_y = pad_y;

        if is_active && mouse_captured {
            if let Some((mx, my)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
                if window.get_mouse_down(minifb::MouseButton::Left) { mouse_btn |= 1; }
                if window.get_mouse_down(minifb::MouseButton::Right) { mouse_btn |= 2; }

                mouse_x = (mx.round() as i32).clamp(0, 767);
                mouse_y = (my.round() as i32).clamp(0, 559);
                prev_mx = mx;
                prev_my = my;
            }
        }

        if m.bus.slave.device_mouse_updated {
            pad_x = m.bus.slave.device_mouse_x;
            pad_y = m.bus.slave.device_mouse_y;
            pad_x_f = pad_x as f32;
            pad_y_f = pad_y as f32;
            m.bus.slave.device_mouse_updated = false;
        }

        if !disc_launched && m.bus.cdrom.is_some() {
            // During automatic boot sequence, do not let host mouse drift override PLAY CD-I auto-click
        } else {
            match input_device_mode {
                InputDeviceMode::Gamepad => {
                    const PAD_SPEED: f32 = 2.5;
                    let mut moved = false;
                    if dir_x != 0 {
                        pad_x_f = (pad_x_f + (dir_x.clamp(-1, 1) as f32) * PAD_SPEED).clamp(0.0, 767.0);
                        pad_x = pad_x_f.round() as i32;
                        moved = true;
                    }
                    if dir_y != 0 {
                        pad_y_f = (pad_y_f + (dir_y.clamp(-1, 1) as f32) * PAD_SPEED).clamp(0.0, 559.0);
                        pad_y = pad_y_f.round() as i32;
                        moved = true;
                    }
                    if moved || key_btn != prev_key_btn {
                        m.bus.slave.update_mouse(pad_x, pad_y, key_btn);
                    }
                    prev_key_btn = key_btn;
                }
                InputDeviceMode::Mouse => {
                    pad_x = mouse_x;
                    pad_y = mouse_y;
                    pad_x_f = mouse_x as f32;
                    pad_y_f = mouse_y as f32;
                    m.bus.slave.update_mouse(pad_x, pad_y, mouse_btn | key_btn);
                }
            }
        }

        // Screenshot hotkey (F12)
        let s_pressed = window.is_key_down(Key::F12);
        if s_pressed && !prev_s {
            save_screenshot(frame_count, &m.bus.mcd212.framebuffer[..]);
        }
        prev_s = s_pressed;

        // Audio hotkeys (M = Mute/Unmute, + / - = Volume)
        let m_pressed = window.is_key_down(Key::M);
        if m_pressed && !prev_m {
            m.bus.cdic.audio.muted = !m.bus.cdic.audio.muted;
            info!("Audio: {}", if m.bus.cdic.audio.muted { "MUTED" } else { "UNMUTED" });
        }
        prev_m = m_pressed;

        if window.is_key_down(Key::Equal) || window.is_key_down(Key::NumPadPlus) {
            m.bus.cdic.audio.volume = (m.bus.cdic.audio.volume + 0.02).min(2.0);
        }
        if window.is_key_down(Key::Minus) || window.is_key_down(Key::NumPadMinus) {
            m.bus.cdic.audio.volume = (m.bus.cdic.audio.volume - 0.02).max(0.0);
        }

        // Precision host frame pacing (prevents fast-forward catchup bursts and frame-time oscillation)
        let target_frame_duration = m.bus.mcd212.video_mode.frame_duration();
        window.update_with_buffer(&m.bus.mcd212.framebuffer[..], SCREEN_WIDTH, SCREEN_HEIGHT)?;
        let now = std::time::Instant::now();

        if now < next_frame_time {
            let remaining = next_frame_time - now;
            if remaining > std::time::Duration::from_millis(2) {
                std::thread::sleep(remaining - std::time::Duration::from_millis(1));
            }
            while std::time::Instant::now() < next_frame_time {
                std::hint::spin_loop();
            }
            next_frame_time += target_frame_duration;
        } else {
            // Drop backlog immediately so emulation never bursts at host CPU speed
            next_frame_time = now + target_frame_duration;
        }

        frame_count += 1;
        let fps_interval = if m.bus.mcd212.video_mode == VideoMode::Pal { 25 } else { 30 };
        if frame_count % fps_interval == 0 {
            let elapsed_secs = fps_timer.elapsed().as_secs_f64();
            if elapsed_secs > 0.0 {
                measured_fps = (fps_interval as f64) / elapsed_secs;
                let inst_delta = m.instructions.saturating_sub(prev_instructions);
                measured_mips = (inst_delta as f64 / elapsed_secs) / 1_000_000.0;
                prev_instructions = m.instructions;

                let curr_sectors = m.bus.cdic.curr_lba;
                let sec_delta = curr_sectors.saturating_sub(prev_cd_sectors);
                measured_cd_rate = sec_delta as f64 / elapsed_secs;
                prev_cd_sectors = curr_sectors;

                measured_render_fps = (buffer_flips_interval as f64) / elapsed_secs;
                buffer_flips_interval = 0;
            }
            fps_timer = std::time::Instant::now();

            let target_fps = m.bus.mcd212.video_mode.fps();
            let speed_pct = (measured_fps / target_fps) * 100.0;

            // Real-Time Anomaly Detector
            if frame_count > 100 && (speed_pct > 108.0 || (speed_pct < 92.0 && speed_pct > 20.0)) {
                log::warn!(
                    "[TELEMETRY SPEED ANOMALY] Speed: {:.1}% ({:.1} FPS on target {:.0} Hz) | MIPS: {:.2} | CD: {:.1} sec/s",
                    speed_pct, measured_fps, target_fps, measured_mips, measured_cd_rate
                );
            }
            if measured_mips > 4.5 && m.bus.emulate_wait_states {
                log::warn!(
                    "[TELEMETRY CPU ANOMALY] Unexpected CPU throughput: {:.2} MIPS (expected ~3.2-3.5 MIPS for SCC68070 at 15.5MHz)",
                    measured_mips
                );
            }

            let video_mode = match m.bus.mcd212.video_mode {
                VideoMode::Pal => "50Hz (PAL)",
                VideoMode::Ntsc => "60Hz (NTSC)",
            };
            let wait_status = if m.bus.emulate_wait_states { "ON" } else { "TURBO" };
            let cd_speed_x = measured_cd_rate / 75.0;
            let cd_str = if measured_cd_rate > 0.5 {
                format!("{:.1} sec/s ({:.1}x)", measured_cd_rate, cd_speed_x)
            } else {
                "IDLE".to_string()
            };
            let render_str = if measured_render_fps > 0.5 {
                format!("{:.0} FPS", measured_render_fps)
            } else {
                format!("{:.0} FPS", measured_fps)
            };
            window.set_title(&format!(
                "OpenCDi – {} | {:.0}% ({:.1} FPS {}) | CPU: {:.2} MIPS | CD: {} | Render: {} | WaitStates: {}",
                current_game_name, speed_pct, measured_fps, video_mode, measured_mips, cd_str, render_str, wait_status
            ));
        }

        frames_run += 1;
        if max_frames.is_some_and(|n| frames_run >= n) {
            info!("Stopping after {} frames (--frames)", frames_run);
            break;
        }
    }

    // Restore host cursor and release mouse confinement on exit
    set_cursor_visibility(true, &mut host_cursor_hidden);
    set_mouse_confinement(false, window.get_window_handle(), &mut mouse_clipped);

    #[cfg(target_os = "windows")]
    unsafe {
        timeEndPeriod(1);
    }

    info!("Emulation stopped. Final PC: 0x{:08X}", m.cpu.pc);
    Ok(())
}
