use opencdi::mcd212::VideoMode;
use std::collections::VecDeque;

pub type Frame = (f32, f32);

// CD-i ADPCM output rate; 18900 Hz streams arrive already doubled.
pub const OUT_RATE: u32 = 37800;
const SECTORS_PER_SECOND: u64 = 75;
const SAMPLES_PER_SECTOR: u64 = OUT_RATE as u64 / SECTORS_PER_SECOND;
// Silence put in front of a stream that starts from empty, so sector-sized bursts never run dry (about 54 ms).
pub const PREFILL: usize = 2048;
// Most frames one channel may hold (about 0.43 s); the oldest are dropped beyond it.
pub const FIFO_CAP: usize = 16384;

pub fn frame_cycles(mode: VideoMode) -> u64 {
    mode.total_scanlines() as u64 * mode.cycles_per_scanline() as u64
}

// Field rate on the CD sector clock (75 sectors per second), so one field of audio is exactly what the emulation produces.
pub fn field_rate(mode: VideoMode) -> f64 {
    (SECTORS_PER_SECOND * mode.cycles_per_sector() as u64) as f64 / frame_cycles(mode) as f64
}

// One audio stream at OUT_RATE between the emulation and the frontend.
#[derive(Default)]
pub struct Channel {
    fifo: VecDeque<Frame>,
    primed: bool,
    pub restarts: u64,
    pub dropped: u64,
    pub frames_in: u64,
}

impl Channel {
    const fn new() -> Self {
        Channel { fifo: VecDeque::new(), primed: false, restarts: 0, dropped: 0, frames_in: 0 }
    }

    pub fn push(&mut self, src: &[Frame]) {
        if src.is_empty() {
            return;
        }
        self.frames_in += src.len() as u64;
        if !self.primed {
            self.fifo.resize(self.fifo.len() + PREFILL, (0.0, 0.0));
            self.primed = true;
            self.restarts += 1;
        }
        self.fifo.extend(src.iter().copied());
        if self.fifo.len() > FIFO_CAP {
            let n = self.fifo.len() - FIFO_CAP;
            self.fifo.drain(..n);
            self.dropped += n as u64;
        }
    }

    // Adds up to out.len() frames into out; running dry ends the stream, so the next push prefills again.
    pub fn mix_into(&mut self, out: &mut [Frame]) {
        let n = out.len().min(self.fifo.len());
        for (o, s) in out.iter_mut().zip(self.fifo.drain(..n)) {
            o.0 += s.0;
            o.1 += s.1;
        }
        if n < out.len() {
            self.primed = false;
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.fifo.len()
    }
}

opencdi::impl_state!(Channel { fifo, primed } skip { restarts, dropped, frames_in });

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

// Linear interpolation to OUT_RATE with an exact rational step; plain f32 ops (no FMA), so results are the same on every IEEE-754 target.
#[derive(Default)]
pub struct Resampler {
    rate: u32,
    num: u32,
    den: u32,
    pos: u32,
    prev: Frame,
}

impl Resampler {
    pub fn set_rate(&mut self, rate: u32) {
        if rate == 0 || rate == self.rate {
            return;
        }
        let g = gcd(rate, OUT_RATE);
        (self.rate, self.num, self.den, self.pos) = (rate, rate / g, OUT_RATE / g, 0);
    }

    pub fn process(&mut self, input: &[Frame], out: &mut Vec<Frame>) {
        if self.den == 0 {
            return;
        }
        let den = self.den as f32;
        for &cur in input {
            while self.pos < self.den {
                let t = self.pos as f32 / den;
                out.push((self.prev.0 + (cur.0 - self.prev.0) * t, self.prev.1 + (cur.1 - self.prev.1) * t));
                self.pos += self.num;
            }
            self.pos -= self.den;
            self.prev = cur;
        }
    }
}

opencdi::impl_state!(Resampler { rate, num, den, pos, prev } skip {});

// Hands the frontend exactly one field of audio per retro_run: CDIC main and audio-map streams plus DVC MPEG audio resampled.
pub struct Pacer {
    pub main: Channel,
    pub sfx: Channel,
    pub mpeg: Channel,
    resampler: Resampler,
    rem: u64,
    scratch: Vec<Frame>,
    out: Vec<Frame>,
    pub frames_out: u64,
}

impl Pacer {
    pub const fn new() -> Self {
        Pacer {
            main: Channel::new(),
            sfx: Channel::new(),
            mpeg: Channel::new(),
            resampler: Resampler { rate: 0, num: 0, den: 0, pos: 0, prev: (0.0, 0.0) },
            rem: 0,
            scratch: Vec::new(),
            out: Vec::new(),
            frames_out: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Pacer::new();
    }

    // Frames due this field: 504 per sector times the sectors the field lasts, remainder carried.
    fn field_len(&mut self, mode: VideoMode) -> usize {
        self.rem += SAMPLES_PER_SECTOR * frame_cycles(mode);
        let sector = mode.cycles_per_sector() as u64;
        let n = self.rem / sector;
        self.rem %= sector;
        n as usize
    }

    pub fn push_mpeg(&mut self, pcm: &[Frame], rate: u32) {
        if pcm.is_empty() {
            return;
        }
        self.resampler.set_rate(rate);
        self.scratch.clear();
        self.resampler.process(pcm, &mut self.scratch);
        self.mpeg.push(&self.scratch);
    }

    pub fn field(&mut self, mode: VideoMode) -> &[Frame] {
        let n = self.field_len(mode);
        self.out.clear();
        self.out.resize(n, (0.0, 0.0));
        self.main.mix_into(&mut self.out);
        self.sfx.mix_into(&mut self.out);
        self.mpeg.mix_into(&mut self.out);
        self.frames_out += n as u64;
        &self.out
    }
}

opencdi::impl_state!(Pacer { main, sfx, mpeg, resampler, rem } skip { scratch, out, frames_out });

#[cfg(test)]
mod tests {
    use super::*;
    use opencdi::savestate::{Reader, State};

    #[test]
    fn field_rate_and_lengths_follow_the_sector_clock() {
        let mut p = Pacer::new();
        let pal: usize = (0..1000).map(|_| p.field_len(VideoMode::Pal)).sum();
        assert_eq!(pal, (504u64 * 299_520 * 1000 / 200_000) as usize);
        assert!((field_rate(VideoMode::Pal) - 50.0801).abs() < 1e-4);
        let mut p = Pacer::new();
        let lens: Vec<usize> = (0..4).map(|_| p.field_len(VideoMode::Ntsc)).collect();
        assert_eq!(lens, [629, 629, 630, 629]);
        assert!((field_rate(VideoMode::Ntsc) - 60.0543).abs() < 1e-4);
    }

    #[test]
    fn channel_prefills_pads_and_caps() {
        let mut c = Channel::default();
        c.push(&[(0.5, 0.5); 100]);
        assert_eq!(c.len(), PREFILL + 100);
        let mut out = vec![(0.0, 0.0); PREFILL + 150];
        c.mix_into(&mut out);
        assert_eq!(out[PREFILL - 1], (0.0, 0.0));
        assert_eq!(out[PREFILL], (0.5, 0.5));
        assert_eq!(out[PREFILL + 100], (0.0, 0.0));
        c.push(&[(0.25, 0.25); FIFO_CAP]);
        assert_eq!((c.len(), c.restarts, c.dropped), (FIFO_CAP, 2, PREFILL as u64));
    }

    // Counts rising zero crossings of the left channel.
    fn crossings(s: &[Frame]) -> usize {
        s.windows(2).filter(|w| w[0].0 < 0.0 && w[1].0 >= 0.0).count()
    }

    #[test]
    fn resampler_keeps_pitch_for_mp2_rates() {
        for rate in [32_000u32, 44_100, 48_000, 37_800] {
            let tone = 1000.0f64;
            let input: Vec<Frame> = (0..rate as usize * 2)
                .map(|i| {
                    let v = (2.0 * std::f64::consts::PI * tone * (i as f64 + 0.25) / rate as f64).sin() as f32;
                    (v, v)
                })
                .collect();
            let mut r = Resampler::default();
            r.set_rate(rate);
            let mut out = Vec::new();
            for chunk in input.chunks(1152) {
                r.process(chunk, &mut out);
            }
            let want = OUT_RATE as usize * 2;
            assert!(out.len().abs_diff(want) <= 1, "{} Hz: {} frames, want {}", rate, out.len(), want);
            assert!(crossings(&out[..OUT_RATE as usize]).abs_diff(1000) <= 1, "{} Hz: {} crossings", rate, crossings(&out[..OUT_RATE as usize]));
        }
    }

    #[test]
    fn pacer_state_round_trips() {
        let mut a = Pacer::new();
        a.main.push(&[(0.1, -0.1); 3000]);
        a.push_mpeg(&[(0.2, 0.3); 1152], 44_100);
        a.field(VideoMode::Ntsc);
        let mut buf = Vec::new();
        a.save(&mut buf);
        let mut b = Pacer::new();
        b.load(&mut Reader::new(&buf)).unwrap();
        for _ in 0..8 {
            a.push_mpeg(&[(0.4, 0.1); 1152], 44_100);
            b.push_mpeg(&[(0.4, 0.1); 1152], 44_100);
            assert_eq!(a.field(VideoMode::Ntsc), b.field(VideoMode::Ntsc));
        }
    }
}
