use std::fs;
use std::path::Path;

pub struct Timekeeper {
    pub ram: [u8; 8192],
}

impl Timekeeper {
    pub fn new() -> Self {
        let mut tk = Self { ram: [0; 8192] };
        tk.init_default();
        tk
    }

    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let data = fs::read(path)?;
        let len = data.len().min(8192);
        self.ram[..len].copy_from_slice(&data[..len]);
        Ok(())
    }

    fn init_default(&mut self) {
        // MK48T08 timekeeper registers are at the top (8184..8191)
        // Set standard flags: Calibration = 0, Seconds = 0 (oscillator running, bit 7 = 0)
        self.ram[8184] = 0x00; // Control
        self.ram[8185] = 0x00; // Seconds (bit 7 = 0 enables oscillator)
        self.ram[8186] = 0x00; // Minutes
        self.ram[8187] = 0x12; // Hours (12:00)
        self.ram[8188] = 0x01; // Day of week
        self.ram[8189] = 0x01; // Date
        self.ram[8190] = 0x01; // Month
        self.ram[8191] = 0x95; // Year (1995)
    }

    pub fn read(&self, offset: u32) -> u8 {
        let idx = (offset as usize) % 8192;
        self.ram[idx]
    }

    pub fn write(&mut self, offset: u32, val: u8) {
        let idx = (offset as usize) % 8192;
        self.ram[idx] = val;
    }
}

opencdi::impl_state!(Timekeeper { ram } skip {});
