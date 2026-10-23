// Host front-end state shared by every platform: key bindings, input modes and the GUI action queue.
use std::path::PathBuf;
use std::sync::Mutex;
use log::info;
use minifb::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProfile {
    Standard, // Frecce + Z / X
    Wasd,     // W, A, S, D + J / K
    Numpad,   // Numpad 8, 4, 6, 2 + 1 / 2
    Custom,   // Personalizzato da opencdi_controls.ini
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceMode {
    Gamepad, // Controller CD-i (Gamepad USB / Joystick / Tastiera centrate)
    Mouse,   // Mouse Philips CD-i (Cursore Fisico Host)
}

pub fn key_to_string(key: Key) -> &'static str {
    match key {
        Key::Up => "Up",
        Key::Down => "Down",
        Key::Left => "Left",
        Key::Right => "Right",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Space => "Space",
        Key::Enter => "Enter",
        Key::LeftShift => "LeftShift",
        Key::RightShift => "RightShift",
        Key::LeftCtrl => "LeftCtrl",
        Key::RightCtrl => "RightCtrl",
        Key::LeftAlt => "LeftAlt",
        Key::RightAlt => "RightAlt",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::NumPad0 => "NumPad0",
        Key::NumPad1 => "NumPad1",
        Key::NumPad2 => "NumPad2",
        Key::NumPad3 => "NumPad3",
        Key::NumPad4 => "NumPad4",
        Key::NumPad5 => "NumPad5",
        Key::NumPad6 => "NumPad6",
        Key::NumPad7 => "NumPad7",
        Key::NumPad8 => "NumPad8",
        Key::NumPad9 => "NumPad9",
        _ => "Z",
    }
}

pub fn string_to_key(s: &str) -> Option<Key> {
    let s_upper = s.trim().to_uppercase();
    match s_upper.as_str() {
        "UP" => Some(Key::Up),
        "DOWN" => Some(Key::Down),
        "LEFT" => Some(Key::Left),
        "RIGHT" => Some(Key::Right),
        "A" => Some(Key::A),
        "B" => Some(Key::B),
        "C" => Some(Key::C),
        "D" => Some(Key::D),
        "E" => Some(Key::E),
        "F" => Some(Key::F),
        "G" => Some(Key::G),
        "H" => Some(Key::H),
        "I" => Some(Key::I),
        "J" => Some(Key::J),
        "K" => Some(Key::K),
        "L" => Some(Key::L),
        "M" => Some(Key::M),
        "N" => Some(Key::N),
        "O" => Some(Key::O),
        "P" => Some(Key::P),
        "Q" => Some(Key::Q),
        "R" => Some(Key::R),
        "S" => Some(Key::S),
        "T" => Some(Key::T),
        "U" => Some(Key::U),
        "V" => Some(Key::V),
        "W" => Some(Key::W),
        "X" => Some(Key::X),
        "Y" => Some(Key::Y),
        "Z" => Some(Key::Z),
        "SPACE" => Some(Key::Space),
        "ENTER" | "RETURN" => Some(Key::Enter),
        "LEFTSHIFT" | "SHIFT" => Some(Key::LeftShift),
        "RIGHTSHIFT" => Some(Key::RightShift),
        "LEFTCTRL" | "CTRL" => Some(Key::LeftCtrl),
        "RIGHTCTRL" => Some(Key::RightCtrl),
        "LEFTALT" | "ALT" => Some(Key::LeftAlt),
        "RIGHTALT" => Some(Key::RightAlt),
        "TAB" => Some(Key::Tab),
        "BACKSPACE" => Some(Key::Backspace),
        "NUMPAD0" | "KP0" | "0" => Some(Key::NumPad0),
        "NUMPAD1" | "KP1" | "1" => Some(Key::NumPad1),
        "NUMPAD2" | "KP2" | "2" => Some(Key::NumPad2),
        "NUMPAD3" | "KP3" | "3" => Some(Key::NumPad3),
        "NUMPAD4" | "KP4" | "4" => Some(Key::NumPad4),
        "NUMPAD5" | "KP5" | "5" => Some(Key::NumPad5),
        "NUMPAD6" | "KP6" | "6" => Some(Key::NumPad6),
        "NUMPAD7" | "KP7" | "7" => Some(Key::NumPad7),
        "NUMPAD8" | "KP8" | "8" => Some(Key::NumPad8),
        "NUMPAD9" | "KP9" | "9" => Some(Key::NumPad9),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub profile: InputProfile,
    pub up: Key,
    pub down: Key,
    pub left: Key,
    pub right: Key,
    pub btn1: Key,
    pub btn2: Key,
}

impl KeyBindings {
    pub fn new() -> Self {
        let mut bindings = Self {
            profile: InputProfile::Standard,
            up: Key::Up,
            down: Key::Down,
            left: Key::Left,
            right: Key::Right,
            btn1: Key::Z,
            btn2: Key::X,
        };
        bindings.load_from_ini();
        bindings
    }

    pub fn set_profile(&mut self, profile: InputProfile) {
        self.profile = profile;
        match profile {
            InputProfile::Standard => {
                self.up = Key::Up;
                self.down = Key::Down;
                self.left = Key::Left;
                self.right = Key::Right;
                self.btn1 = Key::Z;
                self.btn2 = Key::X;
            }
            InputProfile::Wasd => {
                self.up = Key::W;
                self.down = Key::S;
                self.left = Key::A;
                self.right = Key::D;
                self.btn1 = Key::J;
                self.btn2 = Key::K;
            }
            InputProfile::Numpad => {
                self.up = Key::NumPad8;
                self.down = Key::NumPad2;
                self.left = Key::NumPad4;
                self.right = Key::NumPad6;
                self.btn1 = Key::NumPad1;
                self.btn2 = Key::NumPad0;
            }
            InputProfile::Custom => {
                // Keep custom key configuration intact
            }
        }
        self.save_to_ini();
    }

    pub fn save_to_ini(&self) {
        let prof_str = match self.profile {
            InputProfile::Standard => "Standard",
            InputProfile::Wasd => "WASD",
            InputProfile::Numpad => "Numpad",
            InputProfile::Custom => "Custom",
        };
        let content = format!(
            "# OpenCDi Controls Configuration\n\
             # Profile: Standard | WASD | Numpad | Custom\n\
             profile={}\n\n\
             # Custom key bindings (active with profile=Custom or as overrides):\n\
             up={}\n\
             down={}\n\
             left={}\n\
             right={}\n\
             btn1={}\n\
             btn2={}\n",
            prof_str,
            key_to_string(self.up),
            key_to_string(self.down),
            key_to_string(self.left),
            key_to_string(self.right),
            key_to_string(self.btn1),
            key_to_string(self.btn2),
        );
        let _ = std::fs::write("opencdi_controls.ini", content);
    }

    pub fn load_from_ini(&mut self) {
        if let Ok(content) = std::fs::read_to_string("opencdi_controls.ini") {
            let mut detected_custom = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                    continue;
                }
                let parts: Vec<&str> = trimmed.split('=').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    match parts[0].to_lowercase().as_str() {
                        "profile" => match parts[1].to_lowercase().as_str() {
                            "wasd" => self.set_profile(InputProfile::Wasd),
                            "numpad" => self.set_profile(InputProfile::Numpad),
                            "custom" => {
                                self.profile = InputProfile::Custom;
                                detected_custom = true;
                            }
                            _ => self.set_profile(InputProfile::Standard),
                        },
                        "up" => if let Some(k) = string_to_key(parts[1]) { self.up = k; detected_custom = true; }
                        "down" => if let Some(k) = string_to_key(parts[1]) { self.down = k; detected_custom = true; }
                        "left" => if let Some(k) = string_to_key(parts[1]) { self.left = k; detected_custom = true; }
                        "right" => if let Some(k) = string_to_key(parts[1]) { self.right = k; detected_custom = true; }
                        "btn1" => if let Some(k) = string_to_key(parts[1]) { self.btn1 = k; detected_custom = true; }
                        "btn2" => if let Some(k) = string_to_key(parts[1]) { self.btn2 = k; detected_custom = true; }
                        _ => {}
                    }
                }
            }
            if detected_custom && self.profile == InputProfile::Custom {
                info!("Comandi personalizzati caricati da opencdi_controls.ini: up={:?}, down={:?}, left={:?}, right={:?}, btn1={:?}, btn2={:?}",
                    self.up, self.down, self.left, self.right, self.btn1, self.btn2);
            }
        } else {
            self.save_to_ini();
        }
    }
}

pub enum GuiAction {
    InsertDisc(PathBuf),
    OpenDisc(PathBuf),
    EjectDisc,
    Reset,
    Exit,
    SetProfile(InputProfile),
    SetInputDevice(InputDeviceMode),
    ReloadControls,
    ToggleReleaseMouse,
    ToggleMute,
    VolumeUp,
    VolumeDown,
    ToggleFullscreen,
    TakeScreenshot,
    SetVideoMode(crate::mcd212::VideoMode),
    ToggleWaitStates,
    ToggleDvc,
    ShowHelp,
    ShowAbout,
}

static PENDING_ACTIONS: Mutex<Vec<GuiAction>> = Mutex::new(Vec::new());

pub fn pop_action() -> Option<GuiAction> {
    let mut actions = PENDING_ACTIONS.lock().unwrap();
    if actions.is_empty() {
        None
    } else {
        Some(actions.remove(0))
    }
}

pub fn push_action(action: GuiAction) {
    PENDING_ACTIONS.lock().unwrap().push(action);
}
