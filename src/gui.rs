use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use log::info;

use crate::frontend::{push_action, GuiAction, InputDeviceMode, InputProfile};

pub const ID_FILE_INSERT: usize = 1000;
pub const ID_FILE_OPEN: usize = 1001;
pub const ID_FILE_RESET: usize = 1002;
pub const ID_FILE_EJECT: usize = 1004;
pub const ID_FILE_EXIT: usize = 1003;

pub const ID_PROFILE_STANDARD: usize = 1010;
pub const ID_PROFILE_WASD: usize = 1011;
pub const ID_PROFILE_NUMPAD: usize = 1012;
pub const ID_PROFILE_CUSTOM: usize = 1015;
pub const ID_CONTROLS_EDIT_INI: usize = 1016;
pub const ID_CONTROLS_RELOAD_INI: usize = 1017;
pub const ID_CONTROLS_INFO: usize = 1013;
pub const ID_CONTROLS_RELEASE_MOUSE: usize = 1014;
pub const ID_INPUT_DEVICE_GAMEPAD: usize = 1060;
pub const ID_INPUT_DEVICE_MOUSE: usize = 1061;

pub const ID_AUDIO_MUTE: usize = 1020;
pub const ID_AUDIO_VOL_UP: usize = 1021;
pub const ID_AUDIO_VOL_DOWN: usize = 1022;

pub const ID_VIDEO_FULLSCREEN: usize = 1030;
pub const ID_VIDEO_SCREENSHOT: usize = 1031;
pub const ID_VIDEO_PAL: usize = 1032;
pub const ID_VIDEO_NTSC: usize = 1033;
pub const ID_EMU_WAITSTATES: usize = 1050;
pub const ID_EMU_DVC: usize = 1051;

pub const ID_HELP_CONTROLS: usize = 1040;
pub const ID_HELP_ABOUT: usize = 1041;

// Win32 Constants
const MF_STRING: u32 = 0x0000_0000;
const MF_POPUP: u32 = 0x0000_0010;
const MF_SEPARATOR: u32 = 0x0000_0800;
const MF_BYCOMMAND: u32 = 0x0000_0000;
const MF_CHECKED: u32 = 0x0000_0008;
const MF_UNCHECKED: u32 = 0x0000_0000;

const WM_COMMAND: u32 = 0x0111;
const GWLP_WNDPROC: i32 = -4;

const MB_OK: u32 = 0x0000_0000;
const MB_YESNO: u32 = 0x0000_0004;
const MB_YESNOCANCEL: u32 = 0x0000_0003;
const MB_ICONINFORMATION: u32 = 0x0000_0040;
const MB_ICONQUESTION: u32 = 0x0000_0020;
const IDYES: i32 = 6;
const IDNO: i32 = 7;

#[repr(C)]
#[allow(non_snake_case)]
pub struct OPENFILENAMEW {
    pub lStructSize: u32,
    pub hwndOwner: *mut std::ffi::c_void,
    pub hInstance: *mut std::ffi::c_void,
    pub lpstrFilter: *const u16,
    pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: u32,
    pub nFilterIndex: u32,
    pub lpstrFile: *mut u16,
    pub nMaxFile: u32,
    pub lpstrFileTitle: *mut u16,
    pub nMaxFileTitle: u32,
    pub lpstrInitialDir: *const u16,
    pub lpstrTitle: *const u16,
    pub Flags: u32,
    pub nFileOffset: u16,
    pub nFileExtension: u16,
    pub lpstrDefExt: *const u16,
    pub lCustData: usize,
    pub lpfnHook: *mut std::ffi::c_void,
    pub lpTemplateName: *const u16,
    pub pvReserved: *mut std::ffi::c_void,
    pub dwReserved: u32,
    pub FlagsEx: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateMenu() -> *mut std::ffi::c_void;
    fn CreatePopupMenu() -> *mut std::ffi::c_void;
    fn AppendMenuW(hmenu: *mut std::ffi::c_void, uflags: u32, uid: usize, lpitem: *const u16) -> i32;
    fn SetMenu(hwnd: *mut std::ffi::c_void, hmenu: *mut std::ffi::c_void) -> i32;
    fn DrawMenuBar(hwnd: *mut std::ffi::c_void) -> i32;
    fn CheckMenuItem(hmenu: *mut std::ffi::c_void, uid: u32, ucheck: u32) -> u32;
    fn MessageBoxW(hwnd: *mut std::ffi::c_void, lptext: *const u16, lpcaption: *const u16, utype: u32) -> i32;
    fn SetWindowLongPtrW(hwnd: *mut std::ffi::c_void, nindex: i32, dwnewlong: isize) -> isize;
    fn CallWindowProcW(prev: isize, hwnd: *mut std::ffi::c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn PostMessageW(hwnd: *mut std::ffi::c_void, msg: u32, wparam: usize, lparam: isize) -> i32;
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

static mut PREV_WNDPROC: isize = 0;
static mut MENU_HANDLE: *mut std::ffi::c_void = std::ptr::null_mut();
static mut OWNER_HWND: *mut std::ffi::c_void = std::ptr::null_mut();

pub fn update_input_device_menu(mode: InputDeviceMode) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_INPUT_DEVICE_GAMEPAD as u32,
                MF_BYCOMMAND | if mode == InputDeviceMode::Gamepad { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_INPUT_DEVICE_MOUSE as u32,
                MF_BYCOMMAND | if mode == InputDeviceMode::Mouse { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

pub fn update_profile_menu(profile: InputProfile) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_STANDARD as u32,
                MF_BYCOMMAND | if profile == InputProfile::Standard { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_WASD as u32,
                MF_BYCOMMAND | if profile == InputProfile::Wasd { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_NUMPAD as u32,
                MF_BYCOMMAND | if profile == InputProfile::Numpad { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_PROFILE_CUSTOM as u32,
                MF_BYCOMMAND | if profile == InputProfile::Custom { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

pub fn update_video_menu(mode: crate::mcd212::VideoMode) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_VIDEO_PAL as u32,
                MF_BYCOMMAND | if mode == crate::mcd212::VideoMode::Pal { MF_CHECKED } else { MF_UNCHECKED },
            );
            CheckMenuItem(
                MENU_HANDLE,
                ID_VIDEO_NTSC as u32,
                MF_BYCOMMAND | if mode == crate::mcd212::VideoMode::Ntsc { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

pub fn update_waitstates_menu(enabled: bool) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_EMU_WAITSTATES as u32,
                MF_BYCOMMAND | if enabled { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

pub fn update_dvc_menu(fitted: bool) {
    unsafe {
        if !MENU_HANDLE.is_null() {
            CheckMenuItem(
                MENU_HANDLE,
                ID_EMU_DVC as u32,
                MF_BYCOMMAND | if fitted { MF_CHECKED } else { MF_UNCHECKED },
            );
            if !OWNER_HWND.is_null() {
                DrawMenuBar(OWNER_HWND);
            }
        }
    }
}

unsafe extern "system" fn custom_wndproc(
    hwnd: *mut std::ffi::c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if msg == WM_COMMAND {
        let menu_id = (wparam & 0xFFFF) as usize;
        match menu_id {
            ID_FILE_INSERT => {
                if let Some(path) = open_file_dialog(hwnd) {
                    push_action(GuiAction::InsertDisc(path));
                }
                return 0;
            }
            ID_FILE_OPEN => {
                if let Some(path) = open_file_dialog(hwnd) {
                    push_action(GuiAction::OpenDisc(path));
                }
                return 0;
            }
            ID_FILE_EJECT => {
                push_action(GuiAction::EjectDisc);
                return 0;
            }
            ID_FILE_RESET => {
                push_action(GuiAction::Reset);
                return 0;
            }
            ID_FILE_EXIT => {
                push_action(GuiAction::Exit);
                return 0;
            }
            ID_PROFILE_STANDARD => {
                push_action(GuiAction::SetProfile(InputProfile::Standard));
                return 0;
            }
            ID_PROFILE_WASD => {
                push_action(GuiAction::SetProfile(InputProfile::Wasd));
                return 0;
            }
            ID_PROFILE_NUMPAD => {
                push_action(GuiAction::SetProfile(InputProfile::Numpad));
                return 0;
            }
            ID_PROFILE_CUSTOM => {
                push_action(GuiAction::SetProfile(InputProfile::Custom));
                return 0;
            }
            ID_INPUT_DEVICE_GAMEPAD => {
                push_action(GuiAction::SetInputDevice(InputDeviceMode::Gamepad));
                return 0;
            }
            ID_INPUT_DEVICE_MOUSE => {
                push_action(GuiAction::SetInputDevice(InputDeviceMode::Mouse));
                return 0;
            }
            ID_CONTROLS_EDIT_INI => {
                let _ = std::process::Command::new("notepad.exe").arg("opencdi_controls.ini").spawn();
                push_action(GuiAction::SetProfile(InputProfile::Custom));
                return 0;
            }
            ID_CONTROLS_RELOAD_INI => {
                push_action(GuiAction::ReloadControls);
                return 0;
            }
            ID_CONTROLS_INFO => {
                show_controls_dialog(hwnd);
                return 0;
            }
            ID_CONTROLS_RELEASE_MOUSE => {
                push_action(GuiAction::ToggleReleaseMouse);
                return 0;
            }
            ID_AUDIO_MUTE => {
                push_action(GuiAction::ToggleMute);
                return 0;
            }
            ID_AUDIO_VOL_UP => {
                push_action(GuiAction::VolumeUp);
                return 0;
            }
            ID_AUDIO_VOL_DOWN => {
                push_action(GuiAction::VolumeDown);
                return 0;
            }
            ID_VIDEO_FULLSCREEN => {
                push_action(GuiAction::ToggleFullscreen);
                return 0;
            }
            ID_VIDEO_SCREENSHOT => {
                push_action(GuiAction::TakeScreenshot);
                return 0;
            }
            ID_VIDEO_PAL => {
                push_action(GuiAction::SetVideoMode(crate::mcd212::VideoMode::Pal));
                return 0;
            }
            ID_VIDEO_NTSC => {
                push_action(GuiAction::SetVideoMode(crate::mcd212::VideoMode::Ntsc));
                return 0;
            }
            ID_EMU_WAITSTATES => {
                push_action(GuiAction::ToggleWaitStates);
                return 0;
            }
            ID_EMU_DVC => {
                push_action(GuiAction::ToggleDvc);
                return 0;
            }
            ID_HELP_CONTROLS => {
                show_controls_dialog(hwnd);
                return 0;
            }
            ID_HELP_ABOUT => {
                show_about_dialog(hwnd);
                return 0;
            }
            _ => {}
        }
    }

    unsafe {
        if PREV_WNDPROC != 0 {
            CallWindowProcW(PREV_WNDPROC, hwnd, msg, wparam, lparam)
        } else {
            0
        }
    }
}

// Queue a menu command so its modal dialog runs from the window procedure, outside winit's event handler
pub fn post_menu_command(hwnd: *mut std::ffi::c_void, menu_id: usize) {
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, WM_COMMAND, menu_id, 0);
        }
    }
}

pub fn attach_gui_menu(
    hwnd: *mut std::ffi::c_void,
    current_profile: InputProfile,
    current_device_mode: InputDeviceMode,
    current_video_mode: crate::mcd212::VideoMode,
) {
    if hwnd.is_null() {
        return;
    }

    unsafe {
        OWNER_HWND = hwnd;

        let main_menu = CreateMenu();
        if main_menu.is_null() {
            return;
        }
        MENU_HANDLE = main_menu;

        // 1. File Menu
        let file_menu = CreatePopupMenu();
        AppendMenuW(file_menu, MF_STRING, ID_FILE_INSERT, to_wide("Insert Disc (Hot-swap)...\tCtrl+I").as_ptr());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_OPEN, to_wide("Open Disc & Restart Console...\tCtrl+O").as_ptr());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_EJECT, to_wide("Eject Disc").as_ptr());
        AppendMenuW(file_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_RESET, to_wide("Restart Console (Reset)\tF5 / H").as_ptr());
        AppendMenuW(file_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(file_menu, MF_STRING, ID_FILE_EXIT, to_wide("Exit\tEsc").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, file_menu as usize, to_wide("File").as_ptr());

        // 2. Controls Menu
        let controls_menu = CreatePopupMenu();

        let device_menu = CreatePopupMenu();
        AppendMenuW(device_menu, MF_STRING, ID_INPUT_DEVICE_GAMEPAD, to_wide("CD-i Gamepad (USB Controller / Keyboard)").as_ptr());
        AppendMenuW(device_menu, MF_STRING, ID_INPUT_DEVICE_MOUSE, to_wide("Philips CD-i Mouse (Host Cursor)").as_ptr());
        AppendMenuW(controls_menu, MF_POPUP, device_menu as usize, to_wide("Input Device").as_ptr());
        AppendMenuW(controls_menu, MF_SEPARATOR, 0, std::ptr::null());

        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_STANDARD, to_wide("Key Profile: Standard (Arrow Keys + Z / X)").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_WASD, to_wide("Key Profile: WASD + J / K (PC Style)").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_NUMPAD, to_wide("Key Profile: Numpad (8 4 6 2 + 1 / 0)").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_PROFILE_CUSTOM, to_wide("Key Profile: Custom (opencdi_controls.ini)").as_ptr());
        AppendMenuW(controls_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_EDIT_INI, to_wide("Edit Controls in Notepad...").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_RELOAD_INI, to_wide("Reload Controls from INI").as_ptr());
        AppendMenuW(controls_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_RELEASE_MOUSE, to_wide("Release / Capture Mouse\tMiddle Click").as_ptr());
        AppendMenuW(controls_menu, MF_STRING, ID_CONTROLS_INFO, to_wide("View Controls Configuration...").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, controls_menu as usize, to_wide("Controls").as_ptr());

        // 3. Audio Menu
        let audio_menu = CreatePopupMenu();
        AppendMenuW(audio_menu, MF_STRING, ID_AUDIO_MUTE, to_wide("Mute / Unmute Audio\tM").as_ptr());
        AppendMenuW(audio_menu, MF_STRING, ID_AUDIO_VOL_UP, to_wide("Volume Up (+)\t+").as_ptr());
        AppendMenuW(audio_menu, MF_STRING, ID_AUDIO_VOL_DOWN, to_wide("Volume Down (-)\t-").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, audio_menu as usize, to_wide("Audio").as_ptr());

        // 4. Video Menu
        let video_menu = CreatePopupMenu();
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_FULLSCREEN, to_wide("Fullscreen\tF11 / Alt+Enter").as_ptr());
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_SCREENSHOT, to_wide("Save Screenshot\tF12").as_ptr());
        AppendMenuW(video_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_PAL, to_wide("PAL (50 Hz) [CD-i 220 Standard]").as_ptr());
        AppendMenuW(video_menu, MF_STRING, ID_VIDEO_NTSC, to_wide("NTSC (60 Hz)").as_ptr());
        AppendMenuW(video_menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(video_menu, MF_STRING | MF_CHECKED, ID_EMU_WAITSTATES, to_wide("Authentic Hardware Speed (Wait-States)").as_ptr());
        AppendMenuW(video_menu, MF_STRING | MF_CHECKED, ID_EMU_DVC, to_wide("Digital Video Cartridge (DVC) [vmpega.rom]").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, video_menu as usize, to_wide("Video").as_ptr());

        // 5. Help Menu
        let help_menu = CreatePopupMenu();
        AppendMenuW(help_menu, MF_STRING, ID_HELP_CONTROLS, to_wide("Controls Guide...").as_ptr());
        AppendMenuW(help_menu, MF_STRING, ID_HELP_ABOUT, to_wide("About OpenCDi...").as_ptr());
        AppendMenuW(main_menu, MF_POPUP, help_menu as usize, to_wide("Help").as_ptr());

        SetMenu(hwnd, main_menu);
        DrawMenuBar(hwnd);

        update_input_device_menu(current_device_mode);
        update_profile_menu(current_profile);
        update_video_menu(current_video_mode);

        // Subclass window procedure
        let prev = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, custom_wndproc as *const () as usize as isize);
        PREV_WNDPROC = prev;
        info!("Win32 Native Menu attached and window subclassed successfully.");
    }
}

pub fn open_file_dialog(owner: *mut std::ffi::c_void) -> Option<PathBuf> {
    let mut file_buf = [0u16; 1024];

    // Filter string: "CD-i Disc Images (*.chd, *.iso, *.bin, *.cue)\0*.chd;*.iso;*.bin;*.cue\0All Files (*.*)\0*.*\0\0"
    let mut filter: Vec<u16> = Vec::new();
    filter.extend(OsStr::new("CD-i Disc Images (*.chd;*.iso;*.bin;*.cue)").encode_wide());
    filter.push(0);
    filter.extend(OsStr::new("*.chd;*.iso;*.bin;*.cue").encode_wide());
    filter.push(0);
    filter.extend(OsStr::new("All Files (*.*)").encode_wide());
    filter.push(0);
    filter.extend(OsStr::new("*.*").encode_wide());
    filter.push(0);
    filter.push(0);

    let title = to_wide("Select CD-i Disc Image (CHD / ISO / BIN / CUE)");

    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file_buf.as_mut_ptr();
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.lpstrTitle = title.as_ptr();
    ofn.Flags = 0x00000800 | 0x00001000 | 0x00000004 | 0x00000008; // OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY | OFN_NOCHANGEDIR

    unsafe {
        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            let os_string = OsString::from_wide(&file_buf[..len]);
            Some(PathBuf::from(os_string))
        } else {
            None
        }
    }
}

pub fn show_controls_dialog(owner: *mut std::ffi::c_void) {
    let text = to_wide(
        "OpenCDi - Philips CD-i Controls Configuration\n\
        =================================================\n\n\
        Quick control profiles available under the 'Controls' menu:\n\n\
        1. STANDARD PROFILE (Default):\n\
           • Directional Movement / D-Pad: Arrow Keys\n\
           • Button 1 (Action / Confirm): [Z]\n\
           • Button 2 (Jump / Action 2): [X]\n\n\
        2. WASD PROFILE (PC Style):\n\
           • Movement: [W] [A] [S] [D]\n\
           • Button 1: [J]  |  Button 2: [K]\n\n\
        3. NUMPAD PROFILE:\n\
           • Movement: [8] [4] [6] [2]\n\
           • Button 1: [1]  |  Button 2: [0]\n\n\
        4. CUSTOM PROFILE:\n\
           • Configurable via 'Controls -> Edit Controls in Notepad...'\n\
           • Map any key (e.g. Space, Enter, Shift, etc.) in opencdi_controls.ini\n\n\
        MOUSE & TOUCH:\n\
           • Move host mouse to move the on-screen pointer or cursor.\n\
           • Left Click = Button 1  |  Right Click = Button 2\n\
           • Middle Click: Lock / release host mouse cursor to/from the window.\n\n\
        USB GAMEPAD / XBOX / PLAYSTATION:\n\
           • D-Pad or Left Analog Stick = Movement\n\
           • Button A / Cross = Button 1\n\
           • Button B / Circle / X = Button 2\n\
           • Start Button = Action / Pause\n\n\
        KEYBOARD SHORTCUTS:\n\
           • F11 or Alt+Enter: Fullscreen\n\
           • F5 or H: Restart console (Reset)\n\
           • P: Pause  |  Space (hold): Fast-forward\n\
           • F8: Screenshot (PPM)  |  F12: Screenshot (BMP)\n\
           • F2 / F4: Save / Load state  |  F6 / F7: State slot (not implemented yet)\n\
           • Ctrl+I: Insert disc  |  Ctrl+O: Open disc image and restart\n\
           • M: Mute / Unmute audio  |  + / - : Adjust volume\n\
           • Esc: Quit"
    );
    let title = to_wide("OpenCDi - Controls & Key Mapping Guide");
    unsafe {
        MessageBoxW(owner, text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
    }
}

pub fn show_about_dialog(owner: *mut std::ffi::c_void) {
    let text = to_wide(
        "OpenCDi - Philips CD-i Emulator\n\
        Version: 0.1.6 (High Performance Edition)\n\
        Built with Rust, winit & softbuffer\n\n\
        Key Emulation Features:\n\
        • Philips SCC68070 Microprocessor & OS-9 CD-RTOS kernel\n\
        • Philips MCD-212 Video Display Controller (Planes A & B, ICA/DCA display lists)\n\
        • CDIC CD-i Controller & ADPCM Audio Decoder (Levels A, B, C stereo/mono)\n\
        • Motorola 68HC05 Slave Microcontroller (HLE keyboard, mouse, RTC, NVRAM)\n\
        • Digital Video Cartridge (DVC / MPEG-1) for Full Motion Video titles\n\
        • Win32 GUI menu with disc hot-swapping and configurable controls\n\
        • Direct USB gamepad (XInput / DirectInput) and precise mouse capture"
    );
    let title = to_wide("About OpenCDi");
    unsafe {
        MessageBoxW(owner, text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
    }
}
