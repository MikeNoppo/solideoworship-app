use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::OsStr;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::image::Handle;
use iced::{Subscription, Task};
use sdw_core::slide::Slide;
use sdw_core::theme::SlideTheme;
use sdw_core::{BackgroundMedia, MediaExt, MediaFit, MediaHash, MediaKind, MediaRef};
use sdw_library::{
    Library, MediaId, MediaSummary, NewMedia, cas_file_path, store_file_in_cas, thumb_file_path,
};
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

#[derive(Debug, Clone)]
pub struct OpenedPlayer {
    player: Arc<Mutex<Option<MediaPlayer>>>,
    opened_target: VideoTarget,
}

impl OpenedPlayer {
    fn new(player: MediaPlayer, opened_target: VideoTarget) -> Self {
        Self {
            player: Arc::new(Mutex::new(Some(player))),
            opened_target,
        }
    }

    fn take(&self) -> Option<MediaPlayer> {
        self.player.lock().ok().and_then(|mut slot| slot.take())
    }
}

struct PendingOpen {
    generation: u64,
    target: VideoTarget,
    synced_auto_play: bool,
    seek_ms: Option<u64>,
    buffered_frame: Option<Arc<VideoFrame>>,
    buffered_status: Option<PlaybackStatus>,
    buffered_error: Option<Box<str>>,
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

#[derive(Clone, Default)]
pub struct CasLocks(Arc<Mutex<HashMap<MediaHash, Weak<Mutex<()>>>>>);

impl CasLocks {
    pub fn with_lock<R>(&self, sha256: &MediaHash, f: impl FnOnce() -> R) -> R {
        let entry = {
            let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
            map.retain(|_, weak| weak.strong_count() > 0);
            if let Some(existing) = map.get(sha256).and_then(Weak::upgrade) {
                existing
            } else {
                let arc = Arc::new(Mutex::new(()));
                map.insert(sha256.clone(), Arc::downgrade(&arc));
                arc
            }
        };
        let _guard = entry.lock().unwrap_or_else(|e| e.into_inner());
        f()
    }
}

pub struct MediaEngine {
    images: ByteLruCache<ImageCacheKey, CachedHandle>,
    in_flight: HashSet<ImageCacheKey>,
    failed_images: HashSet<ImageCacheKey>,
    slots: HashMap<MediaSlot, SlotPlayer>,
    pending_opens: HashMap<MediaSlot, PendingOpen>,
    failed_videos: HashSet<(MediaHash, MediaExt)>,
    next_generation: u64,
    bus: Arc<MediaEventBus>,
    cas_locks: CasLocks,
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
            failed_images: HashSet::new(),
            slots: HashMap::new(),
            pending_opens: HashMap::new(),
            failed_videos: HashSet::new(),
            next_generation: 1,
            bus: MediaEventBus::new(1),
            cas_locks: CasLocks::default(),
        }
    }

    pub fn cas_locks(&self) -> CasLocks {
        self.cas_locks.clone()
    }

    pub fn image_handle(&self, key: &ImageCacheKey) -> Option<&Handle> {
        self.images.peek(key).map(|entry| &entry.handle)
    }

    pub fn store_decoded_image(&mut self, decoded: DecodedImage) {
        self.in_flight.remove(&decoded.key);
        self.failed_images.remove(&decoded.key);
        self.images.insert(
            decoded.key,
            CachedHandle {
                handle: decoded.handle,
                bytes: decoded.bytes,
            },
        );
    }

    pub fn mark_image_failed(&mut self, key: ImageCacheKey) {
        self.in_flight.remove(&key);
        self.failed_images.insert(key);
    }

    pub fn clear_media_failures(&mut self, sha256: &MediaHash) {
        self.failed_images.remove(&ImageCacheKey {
            sha256: sha256.clone(),
            tier: ImageTier::Thumb,
        });
        self.failed_images.remove(&ImageCacheKey {
            sha256: sha256.clone(),
            tier: ImageTier::Stage,
        });
        self.failed_videos.retain(|(hash, _)| hash != sha256);
    }

    pub fn clear_all_failures(&mut self) {
        self.failed_images.clear();
        self.failed_videos.clear();
    }

    pub fn evict_media(&mut self, sha256: &MediaHash) {
        self.clear_media_failures(sha256);
        self.images.remove(&ImageCacheKey {
            sha256: sha256.clone(),
            tier: ImageTier::Thumb,
        });
        self.images.remove(&ImageCacheKey {
            sha256: sha256.clone(),
            tier: ImageTier::Stage,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn request_image<M: Send + 'static>(
        &mut self,
        media_dir: &Path,
        thumb_dir: &Path,
        sha256: MediaHash,
        ext: MediaExt,
        kind: MediaKind,
        tier: ImageTier,
        on_done: impl FnMut(Result<DecodedImage, Box<str>>) -> M + Send + 'static,
    ) -> Option<Task<M>> {
        if kind == MediaKind::Audio || (tier == ImageTier::Stage && kind != MediaKind::Image) {
            return None;
        }

        let key = ImageCacheKey {
            sha256: sha256.clone(),
            tier,
        };
        if self.images.get(&key).is_some()
            || self.failed_images.contains(&key)
            || !self.in_flight.insert(key.clone())
        {
            return None;
        }

        let media_dir = media_dir.to_path_buf();
        let thumb_dir = thumb_dir.to_path_buf();
        let cas_locks = self.cas_locks.clone();
        let task = crate::work::background(move || {
            let decoded = cas_locks.with_lock(&sha256, || {
                load_or_generate_image(&media_dir, &thumb_dir, &sha256, &ext, kind, tier)
            })?;
            let bytes = decoded.byte_len();
            let raw = iced::advanced::graphics::core::Bytes::from_owner(decoded.pixels);
            let handle = Handle::from_rgba(decoded.width, decoded.height, raw);
            Ok(DecodedImage { key, handle, bytes })
        })
        .map(on_done);

        Some(task)
    }

    fn retire_slot(&mut self, slot: MediaSlot) {
        if let Some(old) = self.slots.remove(&slot) {
            retire_player(old.player);
        }
    }

    pub fn sync_slot<M: Send + 'static>(
        &mut self,
        slot: MediaSlot,
        media_dir: &Path,
        target: Option<VideoTarget>,
        on_opened: impl FnOnce(MediaSlot, u64, Outcome<OpenedPlayer>) -> M + Send + 'static,
    ) -> Option<Task<M>> {
        let Some(target) = target else {
            self.pending_opens.remove(&slot);
            self.retire_slot(slot);
            return None;
        };

        if let Some(existing) = self.slots.get_mut(&slot)
            && existing.target.sha256 == target.sha256
            && existing.target.ext == target.ext
        {
            self.pending_opens.remove(&slot);
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
            return None;
        }

        if let Some(pending) = self.pending_opens.get_mut(&slot)
            && pending.target.sha256 == target.sha256
            && pending.target.ext == target.ext
        {
            pending.target.looping = target.looping;
            pending.target.muted = target.muted;
            if pending.synced_auto_play != target.auto_play {
                pending.synced_auto_play = target.auto_play;
                pending.target.auto_play = target.auto_play;
            }
            return None;
        }

        if self
            .failed_videos
            .contains(&(target.sha256.clone(), target.ext.clone()))
        {
            self.pending_opens.remove(&slot);
            self.retire_slot(slot);
            return None;
        }

        self.retire_slot(slot);
        let generation = self.next_generation;
        self.next_generation = self.next_generation.wrapping_add(1);
        self.pending_opens.insert(
            slot,
            PendingOpen {
                generation,
                synced_auto_play: target.auto_play,
                target: target.clone(),
                seek_ms: None,
                buffered_frame: None,
                buffered_status: None,
                buffered_error: None,
            },
        );

        let path = cas_file_path(media_dir, &target.sha256, &target.ext);
        let sink = SlotSink {
            slot,
            generation,
            bus: Arc::clone(&self.bus),
        };
        let mut on_opened = Some(on_opened);
        let task = crate::work::background(move || {
            let player = MediaPlayer::open(path, target.looping, target.muted, false, sink)
                .map_err(|err| err.to_string().into_boxed_str())?;
            Ok(OpenedPlayer::new(player, target))
        })
        .map(move |res| {
            let callback = on_opened
                .take()
                .expect("MediaPlayerOpened mapper called more than once");
            callback(slot, generation, res)
        });

        Some(task)
    }

    pub fn finish_open(
        &mut self,
        slot: MediaSlot,
        generation: u64,
        result: Outcome<OpenedPlayer>,
    ) -> Option<Box<str>> {
        if self.pending_opens.get(&slot).map(|p| p.generation) != Some(generation) {
            if let Ok(opened) = result
                && let Some(player) = opened.take()
            {
                retire_player(player);
            }
            return None;
        }

        let pending = self.pending_opens.remove(&slot)?;
        match result {
            Ok(opened) => {
                let player = opened.take()?;
                if pending.target.looping != opened.opened_target.looping {
                    player.set_looping(pending.target.looping);
                }
                if pending.target.muted != opened.opened_target.muted {
                    player.set_muted(pending.target.muted);
                }
                if let Some(ms) = pending.seek_ms {
                    player.seek(ms);
                }
                self.retire_slot(slot);
                let auto_play = pending.target.auto_play;
                let duration_ms = player.probe().duration_ms;
                let status = pending.buffered_status.unwrap_or(PlaybackStatus {
                    duration_ms,
                    muted: pending.target.muted,
                    looping: pending.target.looping,
                    ..PlaybackStatus::default()
                });
                self.slots.insert(
                    slot,
                    SlotPlayer {
                        synced_looping: pending.target.looping,
                        synced_muted: pending.target.muted,
                        target: pending.target,
                        generation,
                        player,
                        frame: pending.buffered_frame,
                        status,
                        error: pending.buffered_error,
                        seek_debouncer: SeekDebouncer::new(SEEK_DEBOUNCE),
                    },
                );
                if auto_play {
                    self.play(slot);
                }
                None
            }
            Err(err) => {
                self.failed_videos
                    .insert((pending.target.sha256, pending.target.ext));
                Some(err)
            }
        }
    }

    pub fn play(&mut self, slot: MediaSlot) {
        if let Some(player) = self.slots.get(&slot) {
            player.player.play();
        } else if let Some(pending) = self.pending_opens.get_mut(&slot) {
            pending.target.auto_play = true;
        }
    }

    pub fn pause(&mut self, slot: MediaSlot) {
        if let Some(player) = self.slots.get(&slot) {
            player.player.pause();
        } else if let Some(pending) = self.pending_opens.get_mut(&slot) {
            pending.target.auto_play = false;
        }
    }

    pub fn stop(&mut self, slot: MediaSlot) {
        if let Some(player) = self.slots.get(&slot) {
            player.player.stop();
        } else if let Some(pending) = self.pending_opens.get_mut(&slot) {
            pending.target.auto_play = false;
            pending.seek_ms = Some(0);
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

    pub fn seek_immediate(&mut self, slot: MediaSlot, target_ms: u64) {
        if let Some(entry) = self.slots.get(&slot) {
            entry.player.seek(target_ms);
        } else if let Some(pending) = self.pending_opens.get_mut(&slot) {
            pending.seek_ms = Some(target_ms);
        }
    }

    pub fn set_muted(&mut self, slot: MediaSlot, muted: bool) {
        if let Some(entry) = self.slots.get_mut(&slot) {
            entry.target.muted = muted;
            entry.status.muted = muted;
            entry.player.set_muted(muted);
        } else if let Some(pending) = self.pending_opens.get_mut(&slot) {
            pending.target.muted = muted;
        }
    }

    pub fn set_looping(&mut self, slot: MediaSlot, looping: bool) {
        if let Some(entry) = self.slots.get_mut(&slot) {
            entry.target.looping = looping;
            entry.status.looping = looping;
            entry.player.set_looping(looping);
        } else if let Some(pending) = self.pending_opens.get_mut(&slot) {
            pending.target.looping = looping;
        }
    }

    pub fn frame(&self, slot: MediaSlot) -> Option<&Arc<VideoFrame>> {
        self.slots.get(&slot).and_then(|entry| entry.frame.as_ref())
    }

    pub fn status(&self, slot: MediaSlot) -> Option<PlaybackStatus> {
        self.slots.get(&slot).map(|entry| entry.status)
    }

    pub fn slot_sha256(&self, slot: MediaSlot) -> Option<&MediaHash> {
        self.slots
            .get(&slot)
            .map(|entry| &entry.target.sha256)
            .or_else(|| self.pending_opens.get(&slot).map(|p| &p.target.sha256))
    }

    pub fn has_active_player(&self) -> bool {
        !self.slots.is_empty() || !self.pending_opens.is_empty()
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
                } else if let Some(pending) = self.pending_opens.get_mut(&slot)
                    && pending.generation == generation
                {
                    pending.buffered_frame = Some(frame);
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
                } else if let Some(pending) = self.pending_opens.get_mut(&slot)
                    && pending.generation == generation
                {
                    pending.buffered_status = Some(status);
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
                } else if let Some(pending) = self.pending_opens.get_mut(&slot)
                    && pending.generation == generation
                {
                    pending.buffered_error = Some(message);
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
                item.kind,
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
) -> bool {
    let Some(plan) = plan else {
        if let Some(task) =
            media_engine.sync_slot(slot, media_dir, None, Message::MediaPlayerOpened)
        {
            tasks.push(task);
        }
        return false;
    };

    let has_cas = cas_file_path(media_dir, &plan.sha256, &plan.ext).is_file();
    let missing = (plan.is_standalone || !require_standalone_for_video) && !has_cas;
    if plan.is_standalone && !has_cas {
        if let Some(task) =
            media_engine.sync_slot(slot, media_dir, None, Message::MediaPlayerOpened)
        {
            tasks.push(task);
        }
        return true;
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
                plan.kind,
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

    if let Some(task) = media_engine.sync_slot(slot, media_dir, target, Message::MediaPlayerOpened)
    {
        tasks.push(task);
    }
    missing
}

fn retire_player(player: MediaPlayer) {
    player.silence_for_close();
    let _ = std::thread::Builder::new()
        .name("sdw-media-close".to_owned())
        .spawn(move || drop(player));
}

fn load_or_generate_image(
    media_dir: &Path,
    thumb_dir: &Path,
    sha256: &MediaHash,
    ext: &MediaExt,
    kind: MediaKind,
    tier: ImageTier,
) -> Outcome<sdw_media::DecodedRgba> {
    match tier {
        ImageTier::Stage => {
            if kind != MediaKind::Image {
                return Err("stage image requested for non-image media".into());
            }
            let cas = cas_file_path(media_dir, sha256, ext);
            decode_image_capped(&cas, STAGE_MAX_WIDTH, STAGE_MAX_HEIGHT)
                .map_err(|err| err.to_string().into_boxed_str())
        }
        ImageTier::Thumb => {
            let thumb = thumb_file_path(thumb_dir, sha256);
            if thumb.is_file() {
                return decode_image_capped(&thumb, 320, 320)
                    .map_err(|err| err.to_string().into_boxed_str());
            }
            let cas = cas_file_path(media_dir, sha256, ext);
            match kind {
                MediaKind::Image => decode_image_capped(&cas, 320, 320)
                    .map_err(|err| err.to_string().into_boxed_str()),
                MediaKind::Video => {
                    sdw_media::generate_video_thumbnail(&cas, &thumb, 320)
                        .map_err(|err| err.to_string().into_boxed_str())?;
                    decode_image_capped(&thumb, 320, 320)
                        .map_err(|err| err.to_string().into_boxed_str())
                }
                MediaKind::Audio => Err("audio media has no thumbnail".into()),
            }
        }
    }
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

pub fn prepare_media_import(
    src: &Path,
    media_dir: &Path,
    thumb_dir: &Path,
    cas_locks: &CasLocks,
) -> Outcome<NewMedia> {
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

    let (kind, width, height, duration_ms) = cas_locks.with_lock(&sha256, || {
        let thumb_dest = thumb_file_path(thumb_dir, &sha256);
        let probed = match ext.as_str() {
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
        Ok::<_, Box<str>>(probed)
    })?;

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

pub fn commit_media_import(
    library: &Library,
    src: &Path,
    media_dir: &Path,
    thumb_dir: &Path,
    new_media: &NewMedia,
    cas_locks: &CasLocks,
) -> Result<MediaSummary, sdw_library::Error> {
    cas_locks.with_lock(&new_media.sha256, || {
        let cas_path = store_file_in_cas(src, media_dir, &new_media.sha256, &new_media.ext)?;
        let thumb_dest = thumb_file_path(thumb_dir, &new_media.sha256);
        if !thumb_dest.is_file() {
            match new_media.kind {
                MediaKind::Image => {
                    sdw_media::generate_image_thumbnail(&cas_path, &thumb_dest, 320)
                        .map_err(io::Error::other)?;
                }
                MediaKind::Video => {
                    sdw_media::generate_video_thumbnail(&cas_path, &thumb_dest, 320)
                        .map_err(io::Error::other)?;
                }
                MediaKind::Audio => {}
            }
        }
        let id = library.upsert_media(new_media)?;
        library.media(id)
    })
}

pub fn delete_media_with_files(
    library: &Library,
    id: MediaId,
    media_dir: &Path,
    thumb_dir: &Path,
    cas_locks: &CasLocks,
) -> Result<Option<MediaSummary>, sdw_library::Error> {
    let existing = match library.media(id) {
        Ok(summary) => summary,
        Err(sdw_library::Error::NotFound) => return Ok(None),
        Err(error) => return Err(error),
    };

    cas_locks.with_lock(&existing.sha256, || {
        let deleted = library.delete_media(id)?;
        if let Some(ref removed) = deleted {
            let _ = fs::remove_file(cas_file_path(media_dir, &removed.sha256, &removed.ext));
            let _ = fs::remove_file(thumb_file_path(thumb_dir, &removed.sha256));
        }
        Ok(deleted)
    })
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use std::{env, fs, process};

    use iced::futures::channel::mpsc;
    use sdw_core::{MediaExt, MediaHash, MediaKind};
    use sdw_library::{cas_file_path, thumb_file_path};
    use sdw_media::{ImageCacheKey, ImageTier, MediaPlayer, PlaybackState, PlaybackStatus};

    use super::{
        MediaEngine, MediaEvent, MediaEventBus, MediaSlot, OpenedPlayer, PendingOpen, SlotSink,
        VideoTarget, load_or_generate_image,
    };

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

    #[test]
    fn failed_image_decode_is_not_retried_until_failures_are_cleared() {
        let dir = env::temp_dir().join(format!("sdw-media-engine-fail-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        let media_dir = dir.join("cas");
        let thumb_dir = dir.join("thumbs");
        fs::create_dir_all(&media_dir).unwrap();
        fs::create_dir_all(&thumb_dir).unwrap();

        let sha =
            MediaHash::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                .unwrap();
        let ext = MediaExt::parse("png").unwrap();
        let key = ImageCacheKey {
            sha256: sha.clone(),
            tier: ImageTier::Thumb,
        };

        let mut engine = MediaEngine::new();
        assert!(
            engine
                .request_image(
                    &media_dir,
                    &thumb_dir,
                    sha.clone(),
                    ext.clone(),
                    MediaKind::Image,
                    ImageTier::Thumb,
                    |_| (),
                )
                .is_some()
        );
        engine.mark_image_failed(key);

        assert!(
            engine
                .request_image(
                    &media_dir,
                    &thumb_dir,
                    sha.clone(),
                    ext.clone(),
                    MediaKind::Image,
                    ImageTier::Thumb,
                    |_| (),
                )
                .is_none()
        );

        engine.clear_media_failures(&sha);
        assert!(
            engine
                .request_image(
                    &media_dir,
                    &thumb_dir,
                    sha,
                    ext,
                    MediaKind::Image,
                    ImageTier::Thumb,
                    |_| (),
                )
                .is_some()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_video_thumbnail_regenerates_via_ffmpeg_instead_of_image_decoder() {
        let dir = env::temp_dir().join(format!("sdw-media-engine-vthumb-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        let media_dir = dir.join("cas");
        let thumb_dir = dir.join("thumbs");
        fs::create_dir_all(&media_dir).unwrap();
        fs::create_dir_all(&thumb_dir).unwrap();

        let sha =
            MediaHash::parse("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                .unwrap();
        let ext = MediaExt::parse("mp4").unwrap();
        let cas = cas_file_path(&media_dir, &sha, &ext);
        fs::create_dir_all(cas.parent().unwrap()).unwrap();

        let ok = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=10:duration=0.3",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&cas)
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let thumb = thumb_file_path(&thumb_dir, &sha);
        assert!(!thumb.exists());

        let decoded = load_or_generate_image(
            &media_dir,
            &thumb_dir,
            &sha,
            &ext,
            MediaKind::Video,
            ImageTier::Thumb,
        )
        .unwrap();
        assert_eq!((decoded.width, decoded.height), (160, 90));
        assert!(thumb.is_file());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_slot_deduplicates_pending_open_and_buffers_early_events() {
        let dir = env::temp_dir().join(format!("sdw-media-engine-sync-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        let media_dir = dir.join("cas");
        fs::create_dir_all(&media_dir).unwrap();

        let sha =
            MediaHash::parse("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
                .unwrap();
        let ext = MediaExt::parse("mp4").unwrap();
        let target = VideoTarget {
            sha256: sha.clone(),
            ext: ext.clone(),
            looping: false,
            muted: true,
            auto_play: false,
        };

        let mut engine = MediaEngine::new();
        assert!(
            engine
                .sync_slot(
                    MediaSlot::Preview,
                    &media_dir,
                    Some(target.clone()),
                    |_, _, _| ()
                )
                .is_some()
        );
        assert!(engine.has_active_player());
        assert_eq!(engine.slot_sha256(MediaSlot::Preview), Some(&sha));

        let mut updated = target;
        updated.looping = true;
        updated.auto_play = true;
        assert!(
            engine
                .sync_slot(
                    MediaSlot::Preview,
                    &media_dir,
                    Some(updated.clone()),
                    |_, _, _| ()
                )
                .is_none()
        );
        engine.pause(MediaSlot::Preview);
        assert!(
            engine
                .sync_slot(MediaSlot::Preview, &media_dir, Some(updated), |_, _, _| ())
                .is_none()
        );
        assert!(!engine.pending_opens[&MediaSlot::Preview].target.auto_play);

        let generation = engine
            .pending_opens
            .get(&MediaSlot::Preview)
            .unwrap()
            .generation;
        engine.apply_event(MediaEvent::Status {
            slot: MediaSlot::Preview,
            generation,
            status: PlaybackStatus {
                state: PlaybackState::Playing,
                position_ms: 120,
                duration_ms: Some(1_000),
                muted: true,
                looping: true,
            },
        });
        assert!(
            engine
                .pending_opens
                .get(&MediaSlot::Preview)
                .unwrap()
                .buffered_status
                .is_some()
        );

        assert!(
            engine
                .finish_open(MediaSlot::Preview, generation, Err("missing file".into()))
                .is_some()
        );
        assert!(!engine.has_active_player());

        assert!(
            engine
                .sync_slot(
                    MediaSlot::Preview,
                    &media_dir,
                    Some(VideoTarget {
                        sha256: sha.clone(),
                        ext,
                        looping: false,
                        muted: true,
                        auto_play: false,
                    }),
                    |_, _, _| (),
                )
                .is_none()
        );

        engine.clear_media_failures(&sha);
        assert!(Arc::strong_count(&engine.bus) >= 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancelled_live_open_never_starts_playback() {
        let dir = env::temp_dir().join(format!("sdw-media-engine-cancel-{}", process::id()));
        let sha =
            MediaHash::parse("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")
                .unwrap();
        let ext = MediaExt::parse("mp4").unwrap();
        let clip = cas_file_path(&dir, &sha, &ext);
        fs::create_dir_all(clip.parent().unwrap()).unwrap();
        let ok = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=10:duration=1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&clip)
            .status()
            .is_ok_and(|status| status.success());
        if !ok {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let mut engine = MediaEngine::new();
        let task = engine
            .sync_slot(
                MediaSlot::Live,
                &dir,
                Some(VideoTarget {
                    sha256: sha,
                    ext,
                    looping: false,
                    muted: false,
                    auto_play: true,
                }),
                |_, _, _| (),
            )
            .unwrap();
        engine.sync_slot(MediaSlot::Live, &dir, None, |_, _, _| ());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let initial_state = loop {
            if let Some(state) = engine
                .bus
                .state
                .lock()
                .unwrap()
                .pending
                .iter()
                .find_map(|event| match event {
                    MediaEvent::Status { status, .. } => Some(status.state),
                    _ => None,
                })
            {
                break state;
            }
            assert!(std::time::Instant::now() < deadline, "player never opened");
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        assert_eq!(initial_state, PlaybackState::Paused);
        drop(task);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn accepted_live_open_respects_latest_autoplay() {
        let dir = env::temp_dir().join(format!("sdw-media-engine-autoplay-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("silence.wav");
        let sample_rate = 8_000_u32;
        let data_bytes = sample_rate * 2 * 2;
        let mut wav = [
            b"RIFF".as_slice(),
            &(36 + data_bytes).to_le_bytes(),
            b"WAVEfmt ",
            &16_u32.to_le_bytes(),
            &1_u16.to_le_bytes(),
            &1_u16.to_le_bytes(),
            &sample_rate.to_le_bytes(),
            &(sample_rate * 2).to_le_bytes(),
            &2_u16.to_le_bytes(),
            &16_u16.to_le_bytes(),
            b"data",
            &data_bytes.to_le_bytes(),
        ]
        .concat();
        wav.resize(wav.len() + data_bytes as usize, 0);
        fs::write(&clip, wav).unwrap();

        let target = VideoTarget {
            sha256: MediaHash::parse(
                "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            )
            .unwrap(),
            ext: MediaExt::parse("wav").unwrap(),
            looping: false,
            muted: true,
            auto_play: true,
        };
        for (pause_during_open, expected) in [
            (false, PlaybackState::Playing),
            (true, PlaybackState::Paused),
        ] {
            let mut engine = MediaEngine::new();
            let slot = MediaSlot::Live;
            let generation = 1;
            engine.pending_opens.insert(
                slot,
                PendingOpen {
                    generation,
                    target: target.clone(),
                    synced_auto_play: target.auto_play,
                    seek_ms: None,
                    buffered_frame: None,
                    buffered_status: None,
                    buffered_error: None,
                },
            );
            if pause_during_open {
                engine.pause(slot);
            }
            let sink = SlotSink {
                slot,
                generation,
                bus: Arc::clone(&engine.bus),
            };
            let player = MediaPlayer::open(clip.clone(), false, true, false, sink).unwrap();
            let opened = OpenedPlayer::new(player, target.clone());
            assert!(engine.finish_open(slot, generation, Ok(opened)).is_none());

            engine.set_looping(slot, true);
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                let events = std::mem::take(&mut engine.bus.state.lock().unwrap().pending);
                for event in events {
                    if let MediaEvent::Status { status, .. } = event
                        && status.looping
                    {
                        assert_eq!(status.state, expected);
                        engine.apply_event(event);
                    }
                }
                if engine
                    .status(slot)
                    .is_some_and(|status| status.state == expected && status.looping)
                {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "player did not report its playback state"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn concurrent_delete_and_reimport_same_hash_never_leaves_row_without_cas_file() {
        use std::sync::{Barrier, Mutex};
        use std::thread;

        let dir = env::temp_dir().join(format!("sdw-media-engine-cas-race-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        let media_dir = dir.join("cas");
        let thumb_dir = dir.join("thumbs");
        fs::create_dir_all(&media_dir).unwrap();
        fs::create_dir_all(&thumb_dir).unwrap();

        let src = dir.join("icon.png");
        fs::write(
            &src,
            include_bytes!("../../../assets/logo/png/app-icon-32.png"),
        )
        .unwrap();

        let library = Arc::new(Mutex::new(sdw_library::Library::open_in_memory().unwrap()));
        let cas_locks = super::CasLocks::default();

        for round in 0..8 {
            let prepared =
                super::prepare_media_import(&src, &media_dir, &thumb_dir, &cas_locks).unwrap();
            let initial = {
                let lib = library.lock().unwrap();
                super::commit_media_import(
                    &lib, &src, &media_dir, &thumb_dir, &prepared, &cas_locks,
                )
                .unwrap()
            };

            let barrier = Arc::new(Barrier::new(2));
            let b_del = Arc::clone(&barrier);
            let lib_del = Arc::clone(&library);
            let locks_del = cas_locks.clone();
            let md_del = media_dir.clone();
            let td_del = thumb_dir.clone();
            let del_handle = thread::spawn(move || {
                b_del.wait();
                let lib = lib_del.lock().unwrap();
                super::delete_media_with_files(&lib, initial.id, &md_del, &td_del, &locks_del)
                    .unwrap();
            });

            barrier.wait();
            let reimported =
                super::prepare_media_import(&src, &media_dir, &thumb_dir, &cas_locks).unwrap();
            del_handle.join().unwrap();

            let summary = {
                let lib = library.lock().unwrap();
                super::commit_media_import(
                    &lib,
                    &src,
                    &media_dir,
                    &thumb_dir,
                    &reimported,
                    &cas_locks,
                )
                .unwrap()
            };

            assert!(
                cas_file_path(&media_dir, &summary.sha256, &summary.ext).is_file(),
                "CAS file missing after round {round}"
            );
            assert!(
                thumb_file_path(&thumb_dir, &summary.sha256).is_file(),
                "thumbnail missing after round {round}"
            );
        }

        let _ = fs::remove_dir_all(&dir);
    }
}
