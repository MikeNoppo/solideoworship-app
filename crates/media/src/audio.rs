use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

struct RingShared {
    capacity: usize,
    mask: usize,
    data: Box<[UnsafeCell<f32>]>,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
    discard_until: AtomicUsize,
    played_frames: AtomicU64,
    muted: AtomicBool,
    paused: AtomicBool,
    silenced: Arc<AtomicBool>,
    consumer_active: AtomicBool,
}

unsafe impl Send for RingShared {}
unsafe impl Sync for RingShared {}

pub struct AudioRingBuffer;

pub struct AudioProducer {
    inner: Arc<RingShared>,
    _not_sync: PhantomData<Cell<()>>,
}

pub struct AudioConsumer {
    inner: Arc<RingShared>,
    _not_sync: PhantomData<Cell<()>>,
}

impl AudioRingBuffer {
    pub fn channel(capacity_pow2: usize) -> (AudioProducer, AudioConsumer) {
        let cap = capacity_pow2.max(1024).next_power_of_two();
        let data = (0..cap).map(|_| UnsafeCell::new(0.0)).collect::<Vec<_>>();
        let shared = Arc::new(RingShared {
            capacity: cap,
            mask: cap - 1,
            data: data.into_boxed_slice(),
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            discard_until: AtomicUsize::new(0),
            played_frames: AtomicU64::new(0),
            muted: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            silenced: Arc::new(AtomicBool::new(false)),
            consumer_active: AtomicBool::new(true),
        });
        (
            AudioProducer {
                inner: Arc::clone(&shared),
                _not_sync: PhantomData,
            },
            AudioConsumer {
                inner: shared,
                _not_sync: PhantomData,
            },
        )
    }
}

impl AudioProducer {
    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }

    pub fn silence_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.inner.silenced)
    }

    pub fn is_consumer_active(&self) -> bool {
        self.inner.consumer_active.load(Ordering::Acquire)
    }

    pub fn set_muted(&self, muted: bool) {
        self.inner.muted.store(muted, Ordering::Release);
    }

    pub fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::Release);
    }

    pub fn reset_clock(&self, played_frames: u64) {
        self.inner
            .played_frames
            .store(played_frames, Ordering::Release);
    }

    pub fn played_frames(&self) -> u64 {
        self.inner.played_frames.load(Ordering::Acquire)
    }

    pub fn flush(&mut self) {
        let target = self.inner.write_pos.load(Ordering::Relaxed);
        self.inner.discard_until.store(target, Ordering::Release);
        if !self.is_consumer_active() {
            self.inner.read_pos.store(target, Ordering::Release);
        }
    }

    pub fn available_read(&self) -> usize {
        let discard = self.inner.discard_until.load(Ordering::Relaxed);
        let write = self.inner.write_pos.load(Ordering::Relaxed);
        let read = self.inner.read_pos.load(Ordering::Acquire);
        let effective = effective_read_pos(read, discard, write);
        write.wrapping_sub(effective)
    }

    pub fn available_write(&self) -> usize {
        let write = self.inner.write_pos.load(Ordering::Relaxed);
        let read = self.inner.read_pos.load(Ordering::Acquire);
        self.inner.capacity.saturating_sub(write.wrapping_sub(read))
    }

    pub fn push(&mut self, samples: &[f32]) -> usize {
        let write = self.inner.write_pos.load(Ordering::Relaxed);
        let read = self.inner.read_pos.load(Ordering::Acquire);
        let free = self.inner.capacity.saturating_sub(write.wrapping_sub(read));
        let count = samples.len().min(free);
        if count == 0 {
            return 0;
        }

        let start = write & self.inner.mask;
        let first = count.min(self.inner.capacity - start);
        let base = self.inner.data.as_ptr() as *mut f32;
        unsafe {
            ptr::copy_nonoverlapping(samples.as_ptr(), base.add(start), first);
            if first < count {
                ptr::copy_nonoverlapping(samples.as_ptr().add(first), base, count - first);
            }
        }

        self.inner
            .write_pos
            .store(write.wrapping_add(count), Ordering::Release);
        count
    }
}

impl AudioConsumer {
    pub fn pop_into(&mut self, out: &mut [f32], channels: usize) -> usize {
        let discard = self.inner.discard_until.load(Ordering::Acquire);
        let write = self.inner.write_pos.load(Ordering::Acquire);
        let old_read = self.inner.read_pos.load(Ordering::Relaxed);
        let mut read = effective_read_pos(old_read, discard, write);

        if self.inner.paused.load(Ordering::Acquire) || self.inner.silenced.load(Ordering::Acquire)
        {
            if read != old_read {
                self.inner.read_pos.store(read, Ordering::Release);
            }
            out.fill(0.0);
            return 0;
        }

        let avail = write.wrapping_sub(read);
        let count = out.len().min(avail);

        if count > 0 {
            let start = read & self.inner.mask;
            let first = count.min(self.inner.capacity - start);
            let base = self.inner.data.as_ptr() as *const f32;
            unsafe {
                ptr::copy_nonoverlapping(base.add(start), out.as_mut_ptr(), first);
                if first < count {
                    ptr::copy_nonoverlapping(base, out.as_mut_ptr().add(first), count - first);
                }
            }
            read = read.wrapping_add(count);
        }

        if read != old_read {
            self.inner.read_pos.store(read, Ordering::Release);
        }

        if count < out.len() {
            out[count..].fill(0.0);
        }
        if self.inner.muted.load(Ordering::Acquire) {
            out.fill(0.0);
        }

        if let Some(frames) = count.checked_div(channels) {
            self.inner
                .played_frames
                .fetch_add(frames as u64, Ordering::AcqRel);
        }
        count
    }
}

impl Drop for AudioConsumer {
    fn drop(&mut self) {
        self.inner.consumer_active.store(false, Ordering::Release);
    }
}

fn effective_read_pos(read: usize, discard: usize, write: usize) -> usize {
    if discard.wrapping_sub(read) <= write.wrapping_sub(read) {
        discard
    } else {
        read
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
    pub fn open(mut consumer: AudioConsumer) -> Option<Self> {
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
                    consumer.pop_into(data, ch_usize);
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
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    use super::AudioRingBuffer;

    #[test]
    fn ring_buffer_pushes_wraps_and_pops_fifo() {
        let (mut tx, mut rx) = AudioRingBuffer::channel(1024);
        let input: Vec<f32> = (0..600).map(|i| i as f32).collect();
        assert_eq!(tx.push(&input), 600);

        let mut out = vec![0.0; 400];
        assert_eq!(rx.pop_into(&mut out, 2), 400);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[399], 399.0);
        assert_eq!(tx.played_frames(), 200);

        let more: Vec<f32> = (600..1300).map(|i| i as f32).collect();
        assert_eq!(tx.push(&more), 700);
        assert_eq!(tx.available_read(), 900);

        let mut rest = vec![0.0; 900];
        assert_eq!(rx.pop_into(&mut rest, 2), 900);
        assert_eq!(rest[0], 400.0);
        assert_eq!(rest[899], 1299.0);
    }

    #[test]
    fn flush_discards_older_samples_and_waits_for_consumer_ack_before_overwriting() {
        let (mut tx, mut rx) = AudioRingBuffer::channel(1024);
        let first: Vec<f32> = (0..1024).map(|i| i as f32).collect();
        assert_eq!(tx.push(&first), 1024);
        assert_eq!(tx.available_write(), 0);

        tx.flush();
        assert_eq!(tx.available_read(), 0);
        assert_eq!(tx.available_write(), 0);
        assert_eq!(tx.push(&[9.0, 10.0]), 0);

        let mut out = vec![-1.0; 4];
        assert_eq!(rx.pop_into(&mut out, 2), 0);
        assert_eq!(out, vec![0.0, 0.0, 0.0, 0.0]);
        assert_eq!(tx.available_write(), 1024);

        assert_eq!(tx.push(&[9.0, 10.0]), 2);
        assert_eq!(rx.pop_into(&mut out, 2), 2);
        assert_eq!(out, vec![9.0, 10.0, 0.0, 0.0]);
    }

    #[test]
    fn flush_while_paused_still_acknowledges_discard_and_drops_on_consumer_exit() {
        let (mut tx, mut rx) = AudioRingBuffer::channel(1024);
        let batch = vec![1.0; 1024];
        assert_eq!(tx.push(&batch), 1024);
        tx.set_paused(true);
        tx.flush();

        let mut out = vec![1.0; 16];
        assert_eq!(rx.pop_into(&mut out, 2), 0);
        assert_eq!(out, vec![0.0; 16]);
        assert_eq!(tx.available_write(), 1024);

        assert_eq!(tx.push(&batch), 1024);
        drop(rx);
        assert!(!tx.is_consumer_active());
        tx.flush();
        assert_eq!(tx.available_write(), 1024);
    }

    #[test]
    fn concurrent_push_flush_and_pop_never_observes_partial_overwrite() {
        let (mut tx, mut rx) = AudioRingBuffer::channel(1024);
        let barrier = Arc::new(Barrier::new(2));
        let done = Arc::new(AtomicBool::new(false));

        let c_barrier = Arc::clone(&barrier);
        let c_done = Arc::clone(&done);
        let consumer = thread::spawn(move || {
            c_barrier.wait();
            let mut out = vec![0.0; 128];
            let mut seen = 0usize;
            while !c_done.load(Ordering::Acquire) {
                let n = rx.pop_into(&mut out, 2);
                seen += n;
            }
            seen
        });

        barrier.wait();
        let payload = vec![42.0; 256];
        for i in 0..500 {
            tx.push(&payload);
            if i % 7 == 0 {
                tx.flush();
            }
        }
        done.store(true, Ordering::Release);
        let _ = consumer.join().unwrap();
    }
}
