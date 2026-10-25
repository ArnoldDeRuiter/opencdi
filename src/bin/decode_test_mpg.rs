#[path = "../mpeg/mod.rs"]
mod mpeg;

use std::fs;
use mpeg::demux::demux_all;
use mpeg::video::Video;

fn save_bmp(path: &str, buffer: &[u32], width: u32, height: u32) {
    let mut data = Vec::with_capacity((width * height * 4 + 54) as usize);
    let file_size = 54 + width * height * 4;
    data.extend_from_slice(b"BM");
    data.extend_from_slice(&file_size.to_le_bytes());
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&54u32.to_le_bytes());
    data.extend_from_slice(&40u32.to_le_bytes());
    data.extend_from_slice(&(width as i32).to_le_bytes());
    data.extend_from_slice(&(-(height as i32)).to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(width * height * 4).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    for &pix in buffer.iter() {
        data.extend_from_slice(&pix.to_le_bytes());
    }
    fs::write(path, data).unwrap();
}

fn main() {
    let data = fs::read("bd13_5365.bin").expect("bd13_5365.bin");
    let streams = demux_all(data);
    println!("Demuxed streams: video len = {}", streams.video.len());
    let mut vid = Video::new(streams.video);
    for frame_idx in 0..10 {
        if let Some(idx) = vid.decode() {
            let frame = vid.frame(idx);
            println!("Decoded frame {}: width={}, height={}", frame_idx, frame.width, frame.height);
            let mut buf = vec![0u32; frame.width * frame.height];
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let r = frame.y.data[y * frame.width + x];
                    // grayscale preview
                    buf[y * frame.width + x] = 0xFF000000 | ((r as u32) << 16) | ((r as u32) << 8) | (r as u32);
                }
            }
            save_bmp(&format!("frame_5365_{}.bmp", frame_idx), &buf, frame.width as u32, frame.height as u32);
        }
    }
}
