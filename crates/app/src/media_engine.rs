use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::OsStr;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::image::Handle;
use iced::{Subscription, Task};
use sdw_core::slide::Slide;
use sdw_core::theme::SlideTheme;
use sdw_core::{BackgroundMedia, MediaExt, MediaFit, MediaHash, MediaKind, MediaRef};
use sdw_library::{MediaSummary, NewMedia, cas_file_path, store_file_in_cas, thumb_file_path};
use sdw_media::{
    ByteLruCache, CachedBytes, ImageCacheKey, ImageTier, MediaPlayer, MediaSink, PlaybackStatus,
    SeekDebouncer, VideoFrame, decode_image_capped,
};

use crate::i18n::t;
use crate::message::Message;
use crate::render::slide::{ImageBackdrop, MediaBackdrop, VideoBackdrop};
use crate::work::{Outcome, describe};

const IMAGE_CACHE_MAX_BYTES: usize = 128 * 1024 * 1024;
const STAGE_MAX_WIDTH: u32 = 1920;
const STAGE_MAX_HEIGHT: u32 = 1080;
const SEEK_DEBOUNCE: Duration = Duration::from_millis(80);
const BUS_PENDING_CAP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaSlot {
    Preview = 1,
    Live = 2,
}

impl MediaSlot {
    pub const fn stream_id(self) -> u64 {
        self as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportSlot {
    Preview = 1,
    LivePanel = 2,
    OutputWindow = 3,
}

impl ViewportSlot {
    pub const fn instance_id(self) -> u64 {
        self as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTarget {
    pub sha256: MediaHash,
    pub ext: MediaExt,
    pub looping: bool,
    pub muted: bool,
    pub auto_play: bool,
}

#[derive(Debug, Clone)]
pub enum MediaEvent {
    Frame {
        slot: MediaSlot,
        generation: u64,
        frame: Arc<VideoFrame>,
    },
    Status {
        slot: MediaSlot,
        generation: u64,
        status: PlaybackStatus,
    },
    Error {
        slot: MediaSlot,
        generation: u64,
        message: Box<str>,
    },
}

#[derive(Clone)]
struct CachedHandle {
    handle: Handle,
    bytes: usize,
}

impl CachedBytes for CachedHandle {
    fn byte_len(&self) -> usize {
        self.bytes
    }
}

struct BusState {
    sender: Option<mpsc::Sender<MediaEvent>>,
    pending: VecDeque<MediaEvent>,
}

struct MediaEventBus {
    id: u64,
    state: Mutex<BusState>,
}

impl MediaEventBus {
    fn new(id: u64) -> Arc<Self> {
        Arc::new(Self {
            id,
            state: Mutex::new(BusState {
                sender: None,
                pending: VecDeque::with_capacity(BUS_PENDING_CAP),
            }),
        })
    }

    fn emit(&self, event: MediaEvent) {
        let Ok(mut guard) = self.state.lock() else {
            return;
        };
        let event = if let Some(sender) = guard.sender.as_mut() {
            match sender.try_send(event) {
                Ok(()) => return,
                Err(err) if err.is_disconnected() => {
                    guard.sender = None;
                    err.into_inner()
                }
                Err(_) => return,
            }
        } else {
            event
        };
        if guard.pending.len() >= BUS_PENDING_CAP {
            guard.pending.pop_front();
        }
        guard.pending.push_back(event);
    }

    fn attach(&self, sender: mpsc::Sender<MediaEvent>) -> Vec<MediaEvent> {
        let Ok(mut guard) = self.state.lock() else {
            return Vec::new();
        };
        guard.sender = Some(sender);
        guard.pending.drain(..).collect()
    }
}

#[derive(Clone)]
struct BusHandle(u64, Arc<MediaEventBus>);

impl Hash for BusHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

struct SlotSink {
    slot: MediaSlot,
    generation: u64,
    bus: Arc<MediaEventBus>,
}

impl MediaSink for SlotSink {
    fn on_frame(&self, frame: Arc<VideoFrame>) {
        self.bus.emit(MediaEvent::Frame {
            slot: self.slot,
            generation: self.generation,
            frame,
        });
    }

    fn on_status(&self, status: PlaybackStatus) {
        self.bus.emit(MediaEvent::Status {
            slot: self.slot,
            generation: self.generation,
            status,
        });
    }

    fn on_error(&self, message: String) {
        self.bus.emit(MediaEvent::Error {
            slot: self.slot,
            generation: self.generation,
            message: message.into_boxed_str(),
        });
    }
}

struct SlotPlayer {
    target: VideoTarget,
    synced_looping: bool,
    synced_muted: bool,
    generation: u64,
    player: MediaPlayer,
    frame: Option<Arc<VideoFrame>>,
    status: PlaybackStatus,
    error: Option<Box<str>>,
    seek_debouncer: SeekDebouncer,
}

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub key: ImageCacheKey,
    pub handle: Handle,
    pub bytes: usize,
}

pub struct MediaEngine {
    images: ByteLruCache<ImageCacheKey, CachedHandle>,
    in_flight: HashSet<ImageCacheKey>,
    slots: HashMap<MediaSlot, SlotPlayer>,
    next_generation: u64,
    bus: Arc<MediaEventBus>,
}

impl Default for MediaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaEngine {
    pub fn new() -> Self {
        Self {
            images: ByteLruCache::new(IMAGE_CACHE_MAX_BYTES),
            in_flight: HashSet::new(),
            slots: HashMap::new(),
            next_generation: 1,
            bus: MediaEventBus::new(1),
        }
    }

    pub fn image_handle(&self, key: &ImageCacheKey) -> Option<&Handle> {
        self.images.peek(key).map(|entry| &entry.handle)
    }

    pub fn store_decoded_image(&mut self, decoded: DecodedImage) {
        self.in_flight.remove(&decoded.key);
        self.images.insert(
            decoded.key,
            CachedHandle {
                handle: decoded.handle,
                bytes: decoded.bytes,
            },
        );
    }

    pub fn clear_in_flight(&mut self, key: &ImageCacheKey) {
        self.in_flight.remove(key);
    }

    pub fn request_image<M: Send + 'static>(
        &mut self,
        media_dir: &Path,
        thumb_dir: &Path,
        sha256: MediaHash,
        ext: MediaExt,
        tier: ImageTier,
        on_done: impl FnMut(Result<DecodedImage, Box<str>>) -> M + Send + 'static,
    ) -> Option<Task<M>> {
        let key = ImageCacheKey {
            sha256: sha256.clone(),
            tier,
        };
        if self.images.get(&key).is_some() || !self.in_flight.insert(key.clone()) {
            return None;
        }

        let path = match tier {
            ImageTier::Thumb => {
                let thumb = sdw_library::thumb_file_path(thumb_dir, &sha256);
                if thumb.is_file() {
                    thumb
                } else {
                    cas_file_path(media_dir, &sha256, &ext)
                }
            }
            ImageTier::Stage => cas_file_path(media_dir, &sha256, &ext),
        };

        let (max_w, max_h) = match tier {
            ImageTier::Thumb => (320, 320),
            ImageTier::Stage => (STAGE_MAX_WIDTH, STAGE_MAX_HEIGHT),
        };

        let task = crate::work::background(move || {
            let decoded = decode_image_capped(&path, max_w, max_h)
                .map_err(|err| err.to_string().into_boxed_str())?;
            let bytes = decoded.byte_len();
            let raw = iced::advanced::graphics::core::Bytes::from_owner(decoded.pixels);
            let handle = Handle::from_rgba(decoded.width, decoded.height, raw);
            Ok(DecodedImage { key, handle, bytes })
        })
        .map(on_done);

        Some(task)
    }

    pub fn sync_slot(
        &mut self,
        slot: MediaSlot,
        media_dir: &Path,
        target: Option<VideoTarget>,
    ) -> Result<(), Box<str>> {
        let Some(target) = target else {
            self.slots.remove(&slot);
            return Ok(());
        };

        if let Some(existing) = self.slots.get_mut(&slot)
            && existing.target.sha256 == target.sha256
            && existing.target.ext == target.ext
        {
            if existing.synced_looping != target.looping {
                existing.player.set_looping(target.looping);
                existing.synced_looping = target.looping;
                existing.target.looping = target.looping;
                existing.status.looping = target.looping;
            }
            if existing.synced_muted != target.muted {
                existing.player.set_muted(target.muted);
                existing.synced_muted = target.muted;
                existing.target.muted = target.muted;
                existing.status.muted = target.muted;
            }
            return Ok(());
        }

        self.slots.remove(&slot);
        let generation = self.next_generation;
        self.next_generation = self.next_generation.wrapping_add(1);

        let path = cas_file_path(media_dir, &target.sha256, &target.ext);
        let sink = SlotSink {
            slot,
            generation,
            bus: Arc::clone(&self.bus),
        };
        let player = MediaPlayer::open(path, target.looping, target.muted, target.auto_play, sink)
            .map_err(|err| err.to_string().into_boxed_str())?;
        let duration_ms = player.probe().duration_ms;

        self.slots.insert(
            slot,
            SlotPlayer {
                synced_looping: target.looping,
                synced_muted: target.muted,
                target: target.clone(),
                generation,
                player,
                frame: None,
                status: PlaybackStatus {
                    duration_ms,
                    muted: target.muted,
                    looping: target.looping,
                    ..PlaybackStatus::default()
                },
                error: None,
                seek_debouncer: SeekDebouncer::new(SEEK_DEBOUNCE),
            },
        );

        Ok(())
    }

    pub fn play(&self, slot: MediaSlot) {
        if let Some(player) = self.slots.get(&slot) {
            player.player.play();
        }
    }

    pub fn pause(&self, slot: MediaSlot) {
        if let Some(player) = self.slots.get(&slot) {
            player.player.pause();
        }
    }

    pub fn stop(&self, slot: MediaSlot) {
        if let Some(player) = self.slots.get(&slot) {
            player.player.stop();
        }
    }

    pub fn seek(&mut self, slot: MediaSlot, target_ms: u64) {
        let Some(entry) = self.slots.get_mut(&slot) else {
            return;
        };
        let clamped = entry
            .status
            .duration_ms
            .map_or(target_ms, |dur| target_ms.min(dur));
        entry.status.position_ms = clamped;
        if let Some(ms) = entry.seek_debouncer.request(clamped, Instant::now()) {
            entry.player.seek(ms);
        }
    }

    pub fn commit_seek(&mut self, slot: MediaSlot) {
        let Some(entry) = self.slots.get_mut(&slot) else {
            return;
        };
        if let Some(ms) = entry.seek_debouncer.flush(Instant::now()) {
            entry.status.position_ms = ms;
            entry.player.seek(ms);
        }
    }

    pub fn seek_immediate(&self, slot: MediaSlot, target_ms: u64) {
        if let Some(entry) = self.slots.get(&slot) {
            entry.player.seek(target_ms);
        }
    }

    pub fn set_muted(&mut self, slot: MediaSlot, muted: bool) {
        if let Some(entry) = self.slots.get_mut(&slot) {
            entry.target.muted = muted;
            entry.status.muted = muted;
            entry.player.set_muted(muted);
        }
    }

    pub fn set_looping(&mut self, slot: MediaSlot, looping: bool) {
        if let Some(entry) = self.slots.get_mut(&slot) {
            entry.target.looping = looping;
            entry.status.looping = looping;
            entry.player.set_looping(looping);
        }
    }

    pub fn frame(&self, slot: MediaSlot) -> Option<&Arc<VideoFrame>> {
        self.slots.get(&slot).and_then(|entry| entry.frame.as_ref())
    }

    pub fn status(&self, slot: MediaSlot) -> Option<PlaybackStatus> {
        self.slots.get(&slot).map(|entry| entry.status)
    }

    pub fn slot_sha256(&self, slot: MediaSlot) -> Option<&MediaHash> {
        self.slots.get(&slot).map(|entry| &entry.target.sha256)
    }

    pub fn has_active_player(&self) -> bool {
        !self.slots.is_empty()
    }

    pub fn apply_event(&mut self, event: MediaEvent) {
        match event {
            MediaEvent::Frame {
                slot,
                generation,
                frame,
            } => {
                if let Some(entry) = self.slots.get_mut(&slot)
                    && entry.generation == generation
                {
                    entry.frame = Some(frame);
                }
            }
            MediaEvent::Status {
                slot,
                generation,
                status,
            } => {
                if let Some(entry) = self.slots.get_mut(&slot)
                    && entry.generation == generation
                {
                    entry.status = status;
                }
            }
            MediaEvent::Error {
                slot,
                generation,
                message,
            } => {
                if let Some(entry) = self.slots.get_mut(&slot)
                    && entry.generation == generation
                {
                    entry.error = Some(message);
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<MediaEvent> {
        if !self.has_active_player() {
            return Subscription::none();
        }
        let handle = BusHandle(self.bus.id, Arc::clone(&self.bus));
        Subscription::run_with(handle, |BusHandle(_, bus)| {
            let bus = Arc::clone(bus);
            iced::stream::channel(32, async move |mut output| {
                let (tx, mut rx) = mpsc::channel(32);
                for pending in bus.attach(tx) {
                    let _ = output.send(pending).await;
                }
                while let Some(event) = rx.next().await {
                    let _ = output.send(event).await;
                }
            })
        })
    }

    pub fn stage_backdrop<'a>(
        &'a self,
        slide: Option<&'a Slide>,
        theme: &'a SlideTheme,
        slot: MediaSlot,
        viewport: ViewportSlot,
    ) -> MediaBackdrop<'a> {
        let Some((media, fit, _, _)) = resolve_slide_media_config(slide, theme) else {
            return MediaBackdrop::default();
        };
        match media.kind {
            MediaKind::Video => {
                if let Some(frame) = self.frame(slot) {
                    return MediaBackdrop {
                        image: None,
                        video: Some(VideoBackdrop {
                            stream_id: slot.stream_id(),
                            instance_id: viewport.instance_id(),
                            frame,
                            fit,
                        }),
                    };
                }
                let thumb_key = ImageCacheKey {
                    sha256: media.sha256.clone(),
                    tier: ImageTier::Thumb,
                };
                MediaBackdrop {
                    image: self
                        .image_handle(&thumb_key)
                        .map(|handle| ImageBackdrop { handle, fit }),
                    video: None,
                }
            }
            MediaKind::Image => {
                let stage_key = ImageCacheKey {
                    sha256: media.sha256.clone(),
                    tier: ImageTier::Stage,
                };
                let thumb_key = ImageCacheKey {
                    sha256: media.sha256.clone(),
                    tier: ImageTier::Thumb,
                };
                let handle = self
                    .image_handle(&stage_key)
                    .or_else(|| self.image_handle(&thumb_key));
                MediaBackdrop {
                    image: handle.map(|handle| ImageBackdrop { handle, fit }),
                    video: None,
                }
            }
            MediaKind::Audio => MediaBackdrop::default(),
        }
    }

    pub fn thumbnail_backdrop<'a>(
        &'a self,
        slide: &'a Slide,
        theme: &'a SlideTheme,
    ) -> MediaBackdrop<'a> {
        let Some((media, fit, _, _)) = resolve_slide_media_config(Some(slide), theme) else {
            return MediaBackdrop::default();
        };
        if media.kind == MediaKind::Audio {
            return MediaBackdrop::default();
        }
        let key = ImageCacheKey {
            sha256: media.sha256.clone(),
            tier: ImageTier::Thumb,
        };
        MediaBackdrop {
            image: self
                .image_handle(&key)
                .map(|handle| ImageBackdrop { handle, fit }),
            video: None,
        }
    }

    pub fn request_thumbnails(
        &mut self,
        items: &[MediaSummary],
        media_dir: &Path,
        thumb_dir: &Path,
    ) -> Task<Message> {
        let mut tasks = Vec::new();
        for item in items {
            if !matches!(item.kind, MediaKind::Image | MediaKind::Video) {
                continue;
            }
            let has_thumb = thumb_file_path(thumb_dir, &item.sha256).is_file();
            let has_cas = has_thumb || cas_file_path(media_dir, &item.sha256, &item.ext).is_file();
            if !has_cas {
                continue;
            }
            let key = ImageCacheKey {
                sha256: item.sha256.clone(),
                tier: ImageTier::Thumb,
            };
            if let Some(task) = self.request_image(
                media_dir,
                thumb_dir,
                item.sha256.clone(),
                item.ext.clone(),
                ImageTier::Thumb,
                move |result| Message::MediaImageDecoded(key.clone(), result),
            ) {
                tasks.push(task);
            }
        }
        Task::batch(tasks)
    }
}

pub struct SlotMediaPlan {
    sha256: MediaHash,
    ext: MediaExt,
    kind: MediaKind,
    looping: bool,
    muted: bool,
    is_standalone: bool,
}

pub fn plan_slot_media(slide: Option<&Slide>, theme: &SlideTheme) -> Option<SlotMediaPlan> {
    let is_standalone = slide.and_then(|s| s.media.as_ref()).is_some();
    let (media, _fit, looping, muted) = resolve_slide_media_config(slide, theme)?;
    Some(SlotMediaPlan {
        sha256: media.sha256.clone(),
        ext: media.ext.clone(),
        kind: media.kind,
        looping,
        muted,
        is_standalone,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_slot_sync(
    media_engine: &mut MediaEngine,
    slot: MediaSlot,
    plan: Option<SlotMediaPlan>,
    media_dir: &Path,
    thumb_dir: &Path,
    auto_play_visible: bool,
    force_muted: bool,
    require_standalone_for_video: bool,
    tasks: &mut Vec<Task<Message>>,
) -> (bool, Option<Box<str>>) {
    let Some(plan) = plan else {
        let err = media_engine.sync_slot(slot, media_dir, None).err();
        return (false, err);
    };

    let has_cas = cas_file_path(media_dir, &plan.sha256, &plan.ext).is_file();
    let missing = (plan.is_standalone || !require_standalone_for_video) && !has_cas;
    if plan.is_standalone && !has_cas {
        let err = media_engine.sync_slot(slot, media_dir, None).err();
        return (true, err);
    }

    let request_stage =
        plan.kind == MediaKind::Image && (plan.is_standalone || !require_standalone_for_video);
    let tiers: &[ImageTier] = if request_stage {
        &[ImageTier::Stage, ImageTier::Thumb]
    } else if matches!(plan.kind, MediaKind::Image | MediaKind::Video) {
        &[ImageTier::Thumb]
    } else {
        &[]
    };
    for &tier in tiers {
        let has_source = has_cas
            || (tier == ImageTier::Thumb && thumb_file_path(thumb_dir, &plan.sha256).is_file());
        if has_source {
            let key = ImageCacheKey {
                sha256: plan.sha256.clone(),
                tier,
            };
            if let Some(task) = media_engine.request_image(
                media_dir,
                thumb_dir,
                plan.sha256.clone(),
                plan.ext.clone(),
                tier,
                move |result| Message::MediaImageDecoded(key.clone(), result),
            ) {
                tasks.push(task);
            }
        }
    }

    let video_allowed = !require_standalone_for_video || plan.is_standalone;
    let target =
        if video_allowed && matches!(plan.kind, MediaKind::Video | MediaKind::Audio) && has_cas {
            Some(VideoTarget {
                sha256: plan.sha256,
                ext: plan.ext,
                looping: plan.looping,
                muted: force_muted || plan.muted,
                auto_play: auto_play_visible && (plan.kind == MediaKind::Video || !force_muted),
            })
        } else {
            None
        };

    let err = media_engine.sync_slot(slot, media_dir, target).err();
    (missing, err)
}

pub fn resolve_slide_media_config<'a>(
    slide: Option<&'a Slide>,
    theme: &'a SlideTheme,
) -> Option<(&'a MediaRef, MediaFit, bool, bool)> {
    if let Some(media) = slide.and_then(|s| s.media.as_ref()) {
        return Some((media, MediaFit::Contain, false, false));
    }
    match theme.background_media.as_ref()? {
        BackgroundMedia::Image { media, fit } => Some((media, *fit, true, true)),
        BackgroundMedia::Video {
            media,
            fit,
            loop_playback,
            muted,
        } => Some((media, *fit, *loop_playback, *muted)),
    }
}

pub fn media_ref(summary: &MediaSummary) -> MediaRef {
    MediaRef {
        sha256: summary.sha256.clone(),
        ext: summary.ext.clone(),
        title: Arc::clone(&summary.title),
        kind: summary.kind,
        width: summary.width,
        height: summary.height,
        duration_ms: summary.duration_ms,
    }
}

pub fn prepare_media_import(src: &Path, media_dir: &Path, thumb_dir: &Path) -> Outcome<NewMedia> {
    let ext_str = src
        .extension()
        .and_then(OsStr::to_str)
        .ok_or_else(|| Box::<str>::from(t().library.unavailable_body))?;
    let ext =
        MediaExt::parse(ext_str).ok_or_else(|| Box::<str>::from(t().library.unavailable_body))?;
    let title_str = src
        .file_stem()
        .and_then(OsStr::to_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(t().schedule.media);
    let title: Arc<str> = Arc::from(title_str);

    let (sha256, byte_size) =
        sdw_media::sha256_file(src).map_err(|err| err.to_string().into_boxed_str())?;
    let thumb_dest = thumb_file_path(thumb_dir, &sha256);

    let (kind, width, height, duration_ms) = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => {
            let (w, h) = sdw_media::generate_image_thumbnail(src, &thumb_dest, 320)
                .map_err(|err| err.to_string().into_boxed_str())?;
            (MediaKind::Image, Some(w), Some(h), None)
        }
        _ => {
            let probe =
                sdw_media::probe_media(src).map_err(|err| err.to_string().into_boxed_str())?;
            let probe = if probe.kind == MediaKind::Video {
                sdw_media::generate_video_thumbnail(src, &thumb_dest, 320)
                    .map_err(|err| err.to_string().into_boxed_str())?
            } else {
                probe
            };
            (probe.kind, probe.width, probe.height, probe.duration_ms)
        }
    };

    store_file_in_cas(src, media_dir, &sha256, &ext).map_err(describe)?;

    Ok(NewMedia {
        sha256,
        kind,
        title,
        ext,
        width,
        height,
        duration_ms,
        byte_size,
    })
}

#[cfg(test)]
mod tests {
    use iced::futures::channel::mpsc;

    use super::{MediaEvent, MediaEventBus, MediaSlot};

    #[test]
    fn event_bus_recovers_and_buffers_when_stream_receiver_drops() {
        let bus = MediaEventBus::new(1);
        let (tx1, rx1) = mpsc::channel(4);
        assert!(bus.attach(tx1).is_empty());
        drop(rx1);

        bus.emit(MediaEvent::Error {
            slot: MediaSlot::Preview,
            generation: 1,
            message: "frame0-buffered".into(),
        });

        let (tx2, _rx2) = mpsc::channel(4);
        let buffered = bus.attach(tx2);
        assert_eq!(buffered.len(), 1);
        assert!(matches!(
            &buffered[0],
            MediaEvent::Error { message, .. } if message.as_ref() == "frame0-buffered"
        ));
    }
}
