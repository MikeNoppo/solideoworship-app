use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

static NEXT_FRAME_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Nv12,
    Yuv420p,
}

impl PixelFormat {
    pub fn buffer_len(self, width: u32, height: u32) -> Option<(usize, usize, usize)> {
        let w = usize::try_from(width).ok()?;
        let h = usize::try_from(height).ok()?;
        let y_len = w.checked_mul(h)?;
        let chroma_h = h.div_ceil(2);
        let chroma_w = w.div_ceil(2);

        match self {
            Self::Nv12 => {
                let uv_stride = chroma_w.checked_mul(2)?;
                let uv_len = uv_stride.checked_mul(chroma_h)?;
                Some((y_len, uv_len, 0))
            }
            Self::Yuv420p => {
                let plane_len = chroma_w.checked_mul(chroma_h)?;
                Some((y_len, plane_len, plane_len))
            }
        }
    }
}

#[derive(Debug)]
struct FrameBuffer {
    format: PixelFormat,
    width: u32,
    height: u32,
    y: Vec<u8>,
    u_or_uv: Vec<u8>,
    v: Vec<u8>,
}

impl FrameBuffer {
    fn new(format: PixelFormat, width: u32, height: u32) -> Option<Self> {
        let (y_len, u_len, v_len) = format.buffer_len(width, height)?;
        Some(Self {
            format,
            width,
            height,
            y: vec![0; y_len],
            u_or_uv: vec![0; u_len],
            v: vec![0; v_len],
        })
    }

    fn matches(&self, format: PixelFormat, width: u32, height: u32) -> bool {
        self.format == format && self.width == width && self.height == height
    }
}

#[derive(Debug)]
pub struct FramePool {
    max_cached: usize,
    buffers: Mutex<Vec<FrameBuffer>>,
}

impl FramePool {
    pub fn new(max_cached: usize) -> Arc<Self> {
        Arc::new(Self {
            max_cached: max_cached.max(1),
            buffers: Mutex::new(Vec::with_capacity(max_cached.max(1))),
        })
    }

    pub fn acquire(
        self: &Arc<Self>,
        format: PixelFormat,
        width: u32,
        height: u32,
    ) -> Option<VideoFrameWriter> {
        let recycled = self.buffers.lock().ok().and_then(|mut pool| {
            pool.iter()
                .rposition(|buf| buf.matches(format, width, height))
                .map(|idx| pool.swap_remove(idx))
        });

        let buffer = match recycled {
            Some(buf) => buf,
            None => FrameBuffer::new(format, width, height)?,
        };

        Some(VideoFrameWriter {
            buffer,
            pool: Arc::clone(self),
        })
    }

    fn release(&self, buffer: FrameBuffer) {
        if let Ok(mut pool) = self.buffers.lock()
            && pool.len() < self.max_cached
        {
            pool.push(buffer);
        }
    }

    #[cfg(test)]
    fn available(&self) -> usize {
        self.buffers.lock().map_or(0, |pool| pool.len())
    }
}

#[derive(Debug)]
pub struct VideoFrameWriter {
    buffer: FrameBuffer,
    pool: Arc<FramePool>,
}

impl VideoFrameWriter {
    pub fn planes_mut(&mut self) -> (&mut [u8], &mut [u8], &mut [u8]) {
        (
            &mut self.buffer.y,
            &mut self.buffer.u_or_uv,
            &mut self.buffer.v,
        )
    }

    pub fn finish(self, pts_ms: u64) -> Arc<VideoFrame> {
        Arc::new(VideoFrame {
            id: NEXT_FRAME_ID.fetch_add(1, Ordering::Relaxed),
            pts_ms,
            buffer: Some(self.buffer),
            pool: Some(self.pool),
        })
    }
}

#[derive(Debug)]
pub struct VideoFrame {
    id: u64,
    pts_ms: u64,
    buffer: Option<FrameBuffer>,
    pool: Option<Arc<FramePool>>,
}

impl VideoFrame {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn format(&self) -> PixelFormat {
        self.buffer
            .as_ref()
            .map_or(PixelFormat::Nv12, |buf| buf.format)
    }

    pub fn width(&self) -> u32 {
        self.buffer.as_ref().map_or(0, |buf| buf.width)
    }

    pub fn height(&self) -> u32 {
        self.buffer.as_ref().map_or(0, |buf| buf.height)
    }

    pub fn pts_ms(&self) -> u64 {
        self.pts_ms
    }

    pub fn y_plane(&self) -> &[u8] {
        self.buffer.as_ref().map_or(&[], |buf| &buf.y)
    }

    pub fn u_or_uv_plane(&self) -> &[u8] {
        self.buffer.as_ref().map_or(&[], |buf| &buf.u_or_uv)
    }

    pub fn v_plane(&self) -> &[u8] {
        self.buffer.as_ref().map_or(&[], |buf| &buf.v)
    }
}

impl Drop for VideoFrame {
    fn drop(&mut self) {
        if let (Some(buffer), Some(pool)) = (self.buffer.take(), self.pool.take()) {
            pool.release(buffer);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackStatus {
    pub state: PlaybackState,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub muted: bool,
    pub looping: bool,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        Self {
            state: PlaybackState::Stopped,
            position_ms: 0,
            duration_ms: None,
            muted: false,
            looping: false,
        }
    }
}

#[derive(Debug)]
pub struct SeekDebouncer {
    min_interval: Duration,
    last_sent: Option<Instant>,
    pending_target_ms: Option<u64>,
}

impl SeekDebouncer {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_sent: None,
            pending_target_ms: None,
        }
    }

    pub fn request(&mut self, target_ms: u64, now: Instant) -> Option<u64> {
        let ready = self
            .last_sent
            .is_none_or(|prev| now.duration_since(prev) >= self.min_interval);
        if ready {
            self.last_sent = Some(now);
            self.pending_target_ms = None;
            Some(target_ms)
        } else {
            self.pending_target_ms = Some(target_ms);
            None
        }
    }

    pub fn flush(&mut self, now: Instant) -> Option<u64> {
        let target = self.pending_target_ms.take()?;
        self.last_sent = Some(now);
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{FramePool, PixelFormat, SeekDebouncer};

    #[test]
    fn frame_pool_recycles_buffers_without_reallocating() {
        let pool = FramePool::new(2);
        let first_ptr = {
            let mut writer = pool.acquire(PixelFormat::Nv12, 16, 16).unwrap();
            let ptr = writer.planes_mut().0.as_ptr();
            let frame = writer.finish(33);
            assert_eq!(frame.pts_ms(), 33);
            assert_eq!(frame.y_plane().len(), 256);
            assert_eq!(frame.u_or_uv_plane().len(), 128);
            ptr
        };

        assert_eq!(pool.available(), 1);

        let writer = pool.acquire(PixelFormat::Nv12, 16, 16).unwrap();
        assert_eq!(writer.buffer.y.as_ptr(), first_ptr);
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn frame_pool_handles_odd_dimensions() {
        let (y, uv, v) = PixelFormat::Nv12.buffer_len(5, 5).unwrap();
        assert_eq!((y, uv, v), (25, 18, 0));

        let (y4, u4, v4) = PixelFormat::Yuv420p.buffer_len(5, 5).unwrap();
        assert_eq!((y4, u4, v4), (25, 9, 9));
    }

    #[test]
    fn seek_debouncer_throttles_scrubbing_and_flushes_latest_target() {
        let mut debouncer = SeekDebouncer::new(Duration::from_millis(100));
        let start = Instant::now();

        assert_eq!(debouncer.request(1000, start), Some(1000));
        assert_eq!(
            debouncer.request(2000, start + Duration::from_millis(20)),
            None
        );
        assert_eq!(
            debouncer.request(3500, start + Duration::from_millis(50)),
            None
        );
        assert_eq!(
            debouncer.flush(start + Duration::from_millis(60)),
            Some(3500)
        );
        assert_eq!(debouncer.flush(start + Duration::from_millis(70)), None);
    }
}
