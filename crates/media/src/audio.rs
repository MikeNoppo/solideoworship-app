use std::cell::UnsafeCell;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

pub struct AudioRingBuffer {
    capacity: usize,
    mask: usize,
    data: Box<[UnsafeCell<f32>]>,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
    discard_until: AtomicUsize,
    played_frames: AtomicU64,
    muted: AtomicBool,
    paused: AtomicBool,
}

unsafe impl Send for AudioRingBuffer {}
unsafe impl Sync for AudioRingBuffer {}

impl AudioRingBuffer {
    pub fn new(capacity_pow2: usize) -> Arc<Self> {
        let cap = capacity_pow2.max(1024).next_power_of_two();
        let data = (0..cap).map(|_| UnsafeCell::new(0.0)).collect::<Vec<_>>();
        Arc::new(Self {
            capacity: cap,
            mask: cap - 1,
            data: data.into_boxed_slice(),
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            discard_until: AtomicUsize::new(0),
            played_frames: AtomicU64::new(0),
            muted: AtomicBool::new(false),
            paused: AtomicBool::new(false),
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Release);
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    pub fn reset_clock(&self, played_frames: u64) {
        self.played_frames.store(played_frames, Ordering::Release);
    }

    pub fn played_frames(&self) -> u64 {
        self.played_frames.load(Ordering::Acquire)
    }

    pub fn flush(&self) {
        let target = self.write_pos.load(Ordering::Acquire);
        self.discard_until.store(target, Ordering::Release);
    }

    pub fn available_read(&self) -> usize {
        let discard = self.discard_until.load(Ordering::Acquire);
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.effective_read_pos(discard, write);
        write.wrapping_sub(read)
    }

    pub fn available_write(&self) -> usize {
        self.capacity.saturating_sub(self.available_read())
    }

    pub fn push(&self, samples: &[f32]) -> usize {
        let discard = self.discard_until.load(Ordering::Acquire);
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.effective_read_pos(discard, write);
        let free = self.capacity.saturating_sub(write.wrapping_sub(read));
        let count = samples.len().min(free);
        if count == 0 {
            return 0;
        }

        let start = write & self.mask;
        let first = count.min(self.capacity - start);
        let base = self.data.as_ptr() as *mut f32;
        unsafe {
            ptr::copy_nonoverlapping(samples.as_ptr(), base.add(start), first);
            if first < count {
                ptr::copy_nonoverlapping(samples.as_ptr().add(first), base, count - first);
            }
        }

        self.write_pos
            .store(write.wrapping_add(count), Ordering::Release);
        count
    }

    pub fn pop_into(&self, out: &mut [f32], channels: usize) -> usize {
        if self.paused.load(Ordering::Acquire) {
            out.fill(0.0);
            return 0;
        }

        let discard = self.discard_until.load(Ordering::Acquire);
        let write = self.write_pos.load(Ordering::Acquire);
        let mut read = self.effective_read_pos(discard, write);
        let avail = write.wrapping_sub(read);
        let count = out.len().min(avail);

        if count > 0 {
            let start = read & self.mask;
            let first = count.min(self.capacity - start);
            let base = self.data.as_ptr() as *const f32;
            unsafe {
                ptr::copy_nonoverlapping(base.add(start), out.as_mut_ptr(), first);
                if first < count {
                    ptr::copy_nonoverlapping(base, out.as_mut_ptr().add(first), count - first);
                }
            }
            read = read.wrapping_add(count);
        }

        self.read_pos.store(read, Ordering::Release);

        if count < out.len() {
            out[count..].fill(0.0);
        }
        if self.muted.load(Ordering::Acquire) {
            out.fill(0.0);
        }

        if let Some(frames) = count.checked_div(channels) {
            self.played_frames
                .fetch_add(frames as u64, Ordering::AcqRel);
        }
        count
    }

    fn effective_read_pos(&self, discard: usize, write: usize) -> usize {
        let read = self.read_pos.load(Ordering::Acquire);
        if discard.wrapping_sub(read) <= write.wrapping_sub(read) {
            discard
        } else {
            read
        }
    }
}

#[cfg(feature = "ffmpeg")]
pub struct AudioOutput {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: u16,
}

#[cfg(feature = "ffmpeg")]
impl AudioOutput {
    pub fn open(ring: Arc<AudioRingBuffer>) -> Option<Self> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let default_cfg = device.default_output_config().ok()?;
        let channels = default_cfg.channels().clamp(1, 2);
        let sample_rate = default_cfg.sample_rate();

        let config = cpal::StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let ch_usize = usize::from(channels);
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| {
                    ring.pop_into(data, ch_usize);
                },
                |_| {},
                None,
            )
            .ok()?;

        stream.play().ok()?;

        Some(Self {
            _stream: stream,
            sample_rate,
            channels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::AudioRingBuffer;

    #[test]
    fn ring_buffer_pushes_wraps_and_pops_fifo() {
        let rb = AudioRingBuffer::new(1024);
        let input: Vec<f32> = (0..600).map(|i| i as f32).collect();
        assert_eq!(rb.push(&input), 600);

        let mut out = vec![0.0; 400];
        assert_eq!(rb.pop_into(&mut out, 2), 400);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[399], 399.0);
        assert_eq!(rb.played_frames(), 200);

        let more: Vec<f32> = (600..1300).map(|i| i as f32).collect();
        assert_eq!(rb.push(&more), 700);
        assert_eq!(rb.available_read(), 900);

        let mut rest = vec![0.0; 900];
        assert_eq!(rb.pop_into(&mut rest, 2), 900);
        assert_eq!(rest[0], 400.0);
        assert_eq!(rest[899], 1299.0);
    }

    #[test]
    fn flush_discards_older_samples_and_keeps_post_flush_samples() {
        let rb = AudioRingBuffer::new(1024);
        assert_eq!(rb.push(&[1.0, 2.0, 3.0, 4.0]), 4);
        rb.flush();
        assert_eq!(rb.available_read(), 0);
        assert_eq!(rb.push(&[9.0, 10.0]), 2);

        let mut out = vec![-1.0; 4];
        assert_eq!(rb.pop_into(&mut out, 2), 2);
        assert_eq!(out, vec![9.0, 10.0, 0.0, 0.0]);
    }
}
