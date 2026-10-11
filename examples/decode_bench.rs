//! Wall time to decode a whole .thd through the C API, the way the engine pulls it.
use std::time::Instant;
use truehd_atmos::types::*;
use truehd_atmos::*;

fn main() {
    let path = std::env::args().nth(1).expect("path to .thd");
    let rounds: usize = std::env::args().nth(2).map_or(5, |s| s.parse().unwrap());
    let data = std::fs::read(path).unwrap();
    let mut best = f64::MAX;
    for _ in 0..rounds {
        let d = truehd_atmos_decoder_create(-1);
        let mut block: Block = unsafe { std::mem::zeroed() };
        let (mut frames, mut checksum) = (0u64, 0xcbf29ce484222325u64);
        let mut mix = |v: u64| { checksum = (checksum ^ v).wrapping_mul(0x100000001b3); };
        let t0 = Instant::now();
        for chunk in data.chunks(4096) {
            unsafe { truehd_atmos_decoder_push(d, chunk.as_ptr(), chunk.len(), 0) };
            while unsafe { truehd_atmos_decoder_pull(d, &mut block) } == OK {
                frames += block.frame_count as u64;
                for v in [block.frame_count as u64, block.channel_count as u64, block.flags as u64,
                          block.pts as u64, block.sample_position, block.input_frame_offset, block.layout_serial as u64] { mix(v); }
                for c in 0..block.channel_count as usize {
                    let plane = unsafe { std::slice::from_raw_parts(*block.channels.add(c), block.frame_count as usize) };
                    for x in plane { mix(x.to_bits() as u64); }
                    mix(unsafe { *block.speakers.add(c) } as u64);
                }
                if !block.metadata.is_null() {
                    let m = unsafe { &*block.metadata };
                    for v in [m.element_count, m.bed_count, m.dynamic_count, m.update_count] { mix(v as u64); }
                    let bytes = unsafe { std::slice::from_raw_parts(m.elements as *const u8, m.element_count as usize * std::mem::size_of::<Element>()) };
                    for b in bytes { mix(*b as u64); }
                    for u in 0..m.update_count as usize {
                        let up = unsafe { &*m.updates.add(u) };
                        mix(up.frame_offset as u64); mix(up.ramp_frames as u64);
                        let bytes = unsafe { std::slice::from_raw_parts(up.elements as *const u8, m.element_count as usize * std::mem::size_of::<Element>()) };
                        for b in bytes { mix(*b as u64); }
                    }
                }
            }
        }
        let dt = t0.elapsed().as_secs_f64();
        best = best.min(dt);
        unsafe { truehd_atmos_decoder_destroy(d) };
        eprintln!("{:.3} s, {} frames ({:.1} s audio, {:.1}x), checksum {checksum:016x}", dt, frames, frames as f64 / 48000.0, frames as f64 / 48000.0 / dt);
    }
    println!("best {:.3} s", best);
}
