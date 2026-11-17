use std::collections::VecDeque;
use std::fmt;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ac_ffmpeg::codec::audio::{AudioDecoder, AudioResampler, ChannelLayout, SampleFormat};
use ac_ffmpeg::codec::video::filter::VideoFilter;
use ac_ffmpeg::codec::video::{
    PixelFormat as FfPixelFormat, VideoDecoder, VideoFrame as FfVideoFrame,
};
use ac_ffmpeg::codec::{AudioCodecParameters, Decoder, Filter};
use ac_ffmpeg::format::demuxer::{Demuxer, DemuxerWithStreamInfo, SeekTarget};
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::time::Timestamp;
use image::ImageEncoder;
use image::codecs::webp::WebPEncoder;
use sdw_core::MediaKind;

use crate::MediaError;
use crate::audio::{AudioConsumer, AudioOutput, AudioProducer, AudioRingBuffer};
use crate::image::{fit_within, temp_thumb_path};
use crate::video::{FramePool, PixelFormat, PlaybackState, PlaybackStatus, VideoFrame};

const AUDIO_RING_SAMPLES: usize = 65_536;
const TARGET_AUDIO_CHANNELS: u32 = 2;
const FALLBACK_AUDIO_RATE: u32 = 48_000;
const FRAME_POOL_SLOTS: usize = 5;
const VIDEO_LOOKAHEAD: usize = 3;
const MAX_VIDEO_AHEAD_ON_AUDIO_SEARCH: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaProbe {
    pub kind: MediaKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
}

pub fn probe_media(path: &Path) -> Result<MediaProbe, MediaError> {
    let demuxer = open_demuxer(path)?;
    Ok(inspect_streams(&demuxer).probe)
}

pub fn generate_video_thumbnail(
    src: &Path,
    dest_webp: &Path,
    max_width: u32,
) -> Result<MediaProbe, MediaError> {
    if max_width == 0 {
        return Err(MediaError::EmptyDimensions);
    }

    let mut demuxer = open_demuxer(src)?;
    let info = inspect_streams(&demuxer);
    let video_idx = info.video_index.ok_or(MediaError::NoVideoStream)?;

    let seek_ms = info
        .probe
        .duration_ms
        .filter(|&dur| dur > 1_000)
        .map(|dur| (dur / 10).min(5_000));
    if let Some(ms) = seek_ms {
        let _ = demuxer.seek_to_timestamp(
            Timestamp::from_millis(i64::try_from(ms).unwrap_or(0)),
            SeekTarget::UpTo,
        );
    }

    let mut frame = decode_first_video_frame(&mut demuxer, video_idx)?;
    if frame.is_none() && seek_ms.is_some() {
        let mut fresh = open_demuxer(src)?;
        frame = decode_first_video_frame(&mut fresh, video_idx)?;
    }

    let frame = frame.ok_or(MediaError::NoVideoStream)?;
    let src_w = u32::try_from(frame.width()).unwrap_or(0);
    let src_h = u32::try_from(frame.height()).unwrap_or(0);
    if src_w == 0 || src_h == 0 {
        return Err(MediaError::EmptyDimensions);
    }

    let (dst_w, dst_h) = fit_within(src_w, src_h, max_width, max_width);
    let mut scaler = VideoFilter::builder(frame.width(), frame.height(), frame.pixel_format())
        .build(&format!(
            "scale={dst_w}:{dst_h}:flags=bilinear:in_color_matrix=auto,format=rgba"
        ))
        .map_err(|err| MediaError::Decoder(err.to_string()))?;
    scaler
        .try_push(frame)
        .map_err(|err| MediaError::Decoder(err.to_string()))?;
    let scaled = scaler
        .take()
        .map_err(|err| MediaError::Decoder(err.to_string()))?
        .ok_or(MediaError::NoVideoStream)?;
    let planes = scaled.planes();
    let plane = &planes[0];
    let row_bytes = (dst_w as usize) * 4;
    let line_size = plane.line_size();
    let raw_plane = plane.data();

    let rgba = if line_size == row_bytes {
        raw_plane[..row_bytes * dst_h as usize].to_vec()
    } else {
        let mut compact = Vec::with_capacity(row_bytes * dst_h as usize);
        for row in 0..dst_h as usize {
            let offset = row * line_size;
            compact.extend_from_slice(&raw_plane[offset..offset + row_bytes]);
        }
        compact
    };

    if let Some(parent) = dest_webp.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = temp_thumb_path(dest_webp);
    let write_result = (|| -> Result<(), MediaError> {
        let file = File::create(&tmp)?;
        let writer = BufWriter::new(file);
        WebPEncoder::new_lossless(writer)
            .write_image(&rgba, dst_w, dst_h, image::ExtendedColorType::Rgba8)
            .map_err(|err| MediaError::ImageDecode(err.to_string()))?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }

    if let Err(err) = fs::rename(&tmp, dest_webp) {
        let _ = fs::remove_file(&tmp);
        if !dest_webp.is_file() {
            return Err(err.into());
        }
    }

    Ok(info.probe)
}

pub trait MediaSink: Send + 'static {
    fn on_frame(&self, frame: Arc<VideoFrame>);
    fn on_status(&self, status: PlaybackStatus);
    fn on_error(&self, message: String);
}

enum PlayerCommand {
    Play,
    Pause,
    Stop,
    Seek(u64),
    SetMuted(bool),
    SetLooping(bool),
    Shutdown,
}

pub struct MediaPlayer {
    tx: Sender<PlayerCommand>,
    silence: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    probe: MediaProbe,
}

impl fmt::Debug for MediaPlayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MediaPlayer")
            .field("probe", &self.probe)
            .finish_non_exhaustive()
    }
}

impl MediaPlayer {
    pub fn open(
        path: PathBuf,
        looping: bool,
        muted: bool,
        auto_play: bool,
        sink: impl MediaSink,
    ) -> Result<Self, MediaError> {
        Self::open_with_audio_output(path, looping, muted, auto_play, sink, |consumer| {
            let output = AudioOutput::open(consumer)?;
            Some((output.sample_rate, u32::from(output.channels), output))
        })
    }

    fn open_with_audio_output<Output>(
        path: PathBuf,
        looping: bool,
        muted: bool,
        auto_play: bool,
        sink: impl MediaSink,
        open_audio: impl FnOnce(AudioConsumer) -> Option<(u32, u32, Output)> + Send + 'static,
    ) -> Result<Self, MediaError> {
        let demuxer = open_demuxer(&path)?;
        let info = inspect_streams(&demuxer);
        let probe = info.probe;
        let (audio_ring, audio_consumer) = AudioRingBuffer::channel(AUDIO_RING_SAMPLES);
        let silence = audio_ring.silence_signal();

        let (tx, rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("sdw-media-player".to_owned())
            .spawn(move || {
                let audio_output = info.audio_index.and_then(|_| open_audio(audio_consumer));
                run_player(
                    demuxer,
                    info,
                    looping,
                    muted,
                    auto_play,
                    audio_output
                        .as_ref()
                        .map(|(rate, channels, _)| (*rate, *channels)),
                    audio_ring,
                    rx,
                    sink,
                );
            })?;

        Ok(Self {
            tx,
            silence,
            worker: Some(worker),
            probe,
        })
    }

    pub fn probe(&self) -> MediaProbe {
        self.probe
    }

    pub fn play(&self) {
        let _ = self.tx.send(PlayerCommand::Play);
    }

    pub fn pause(&self) {
        let _ = self.tx.send(PlayerCommand::Pause);
    }

    pub fn stop(&self) {
        let _ = self.tx.send(PlayerCommand::Stop);
    }

    pub fn seek(&self, target_ms: u64) {
        let _ = self.tx.send(PlayerCommand::Seek(target_ms));
    }

    pub fn set_muted(&self, muted: bool) {
        let _ = self.tx.send(PlayerCommand::SetMuted(muted));
    }

    pub fn set_looping(&self, looping: bool) {
        let _ = self.tx.send(PlayerCommand::SetLooping(looping));
    }

    pub fn silence_for_close(&self) {
        self.silence.store(true, Ordering::Release);
        let _ = self.tx.send(PlayerCommand::Shutdown);
    }
}

impl Drop for MediaPlayer {
    fn drop(&mut self) {
        self.silence.store(true, Ordering::Release);
        let _ = self.tx.send(PlayerCommand::Shutdown);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

struct StreamInfo {
    probe: MediaProbe,
    video_index: Option<usize>,
    audio_index: Option<usize>,
}

fn open_demuxer(path: &Path) -> Result<DemuxerWithStreamInfo<File>, MediaError> {
    let file = File::open(path)?;
    let io = IO::from_seekable_read_stream(file);
    Demuxer::builder()
        .build(io)
        .map_err(|err| MediaError::Decoder(err.to_string()))?
        .find_stream_info(None)
        .map_err(|(_, err)| MediaError::Decoder(err.to_string()))
}

fn inspect_streams(demuxer: &DemuxerWithStreamInfo<File>) -> StreamInfo {
    let mut video_index = None;
    let mut audio_index = None;
    let mut width = None;
    let mut height = None;
    let mut duration_ms = None;

    for (idx, stream) in demuxer.streams().iter().enumerate() {
        let params = stream.codec_parameters();
        let stream_dur = stream
            .duration()
            .as_millis()
            .and_then(|ms| u64::try_from(ms).ok());

        if video_index.is_none() && params.is_video_codec() {
            if let Some(vparams) = params.into_video_codec_parameters() {
                let is_still = matches!(
                    vparams.decoder_name(),
                    Some("png" | "mjpeg" | "webp" | "bmp" | "gif")
                ) && stream.frames() == Some(1);
                if !is_still {
                    video_index = Some(idx);
                    width = u32::try_from(vparams.width()).ok().filter(|&w| w > 0);
                    height = u32::try_from(vparams.height()).ok().filter(|&h| h > 0);
                    duration_ms = stream_dur;
                }
            }
        } else if audio_index.is_none() && params.is_audio_codec() {
            audio_index = Some(idx);
            if duration_ms.is_none() {
                duration_ms = stream_dur;
            }
        }
    }

    let kind = if video_index.is_some() {
        MediaKind::Video
    } else if audio_index.is_some() {
        MediaKind::Audio
    } else {
        MediaKind::Image
    };

    StreamInfo {
        probe: MediaProbe {
            kind,
            width,
            height,
            duration_ms,
        },
        video_index,
        audio_index,
    }
}

fn decode_first_video_frame(
    demuxer: &mut DemuxerWithStreamInfo<File>,
    video_idx: usize,
) -> Result<Option<FfVideoFrame>, MediaError> {
    let mut decoder = VideoDecoder::from_stream(&demuxer.streams()[video_idx])
        .map_err(|err| MediaError::Decoder(err.to_string()))?
        .build()
        .map_err(|err| MediaError::Decoder(err.to_string()))?;

    while let Some(packet) = demuxer
        .take()
        .map_err(|err| MediaError::Decoder(err.to_string()))?
    {
        if packet.stream_index() != video_idx {
            continue;
        }
        decoder
            .push(packet)
            .map_err(|err| MediaError::Decoder(err.to_string()))?;
        if let Some(frame) = decoder
            .take()
            .map_err(|err| MediaError::Decoder(err.to_string()))?
        {
            return Ok(Some(frame));
        }
    }

    decoder
        .flush()
        .map_err(|err| MediaError::Decoder(err.to_string()))?;
    decoder
        .take()
        .map_err(|err| MediaError::Decoder(err.to_string()))
}

struct PlayerDecoders {
    video: Option<VideoDecoder>,
    audio: Option<AudioDecoder>,
    resampler: Option<AudioResampler>,
    scaler: Option<(FfPixelFormat, usize, usize, VideoFilter)>,
    sample_rate: u32,
    channels: u32,
}

impl PlayerDecoders {
    fn new(
        demuxer: &DemuxerWithStreamInfo<File>,
        info: &StreamInfo,
        output_rate: u32,
        output_channels: u32,
    ) -> Result<Self, MediaError> {
        let video = info
            .video_index
            .map(|idx| {
                VideoDecoder::from_stream(&demuxer.streams()[idx])
                    .map_err(|err| MediaError::Decoder(err.to_string()))?
                    .set_option("threads", "4")
                    .build()
                    .map_err(|err| MediaError::Decoder(err.to_string()))
            })
            .transpose()?;

        let (audio, resampler) = if let Some(idx) = info.audio_index {
            let params: AudioCodecParameters = demuxer.streams()[idx]
                .codec_parameters()
                .into_audio_codec_parameters()
                .ok_or_else(|| MediaError::Decoder("missing audio codec parameters".to_owned()))?;
            let decoder = AudioDecoder::from_stream(&demuxer.streams()[idx])
                .map_err(|err| MediaError::Decoder(err.to_string()))?
                .build()
                .map_err(|err| MediaError::Decoder(err.to_string()))?;
            let target_layout = ChannelLayout::from_channels(output_channels)
                .ok_or_else(|| MediaError::Decoder("invalid output channel layout".to_owned()))?;
            let target_fmt: SampleFormat = "flt"
                .parse()
                .map_err(|_| MediaError::Decoder("unsupported flt sample format".to_owned()))?;
            let resampler = AudioResampler::builder()
                .source_channel_layout(params.channel_layout().to_owned())
                .source_sample_format(params.sample_format())
                .source_sample_rate(params.sample_rate())
                .target_channel_layout(target_layout)
                .target_sample_format(target_fmt)
                .target_sample_rate(output_rate)
                .build()
                .map_err(|err| MediaError::Decoder(err.to_string()))?;
            (Some(decoder), Some(resampler))
        } else {
            (None, None)
        };

        Ok(Self {
            video,
            audio,
            resampler,
            scaler: None,
            sample_rate: output_rate,
            channels: output_channels,
        })
    }

    fn rebuild(
        &mut self,
        demuxer: &DemuxerWithStreamInfo<File>,
        info: &StreamInfo,
    ) -> Result<(), MediaError> {
        *self = Self::new(demuxer, info, self.sample_rate, self.channels)?;
        Ok(())
    }
}

struct PlayerContext<S: MediaSink> {
    demuxer: DemuxerWithStreamInfo<File>,
    info: StreamInfo,
    decoders: PlayerDecoders,
    audio_ring: AudioProducer,
    use_audio_clock: bool,
    audio_start_ms: Option<u64>,
    paused_seeked: bool,
    output_rate: u32,
    pool: Arc<FramePool>,
    video_queue: VecDeque<Arc<VideoFrame>>,
    audio_backlog: Vec<f32>,
    audio_backlog_offset: usize,
    eof: bool,
    finished: bool,
    state: PlaybackState,
    muted: bool,
    looping: bool,
    seek_origin_ms: u64,
    clock_origin_ms: u64,
    current_pos_ms: u64,
    play_started_at: Instant,
    sink: S,
}

impl<S: MediaSink> PlayerContext<S> {
    fn emit_status(&self) {
        self.sink.on_status(PlaybackStatus {
            state: self.state,
            position_ms: self.current_pos_ms,
            duration_ms: self.info.probe.duration_ms,
            muted: self.muted,
            looping: self.looping,
        });
    }

    fn clock_ms(&self) -> u64 {
        if self.use_audio_clock {
            self.clock_origin_ms.saturating_add(
                self.audio_ring.played_frames().saturating_mul(1_000)
                    / u64::from(self.output_rate.max(1)),
            )
        } else {
            self.clock_origin_ms.saturating_add(
                u64::try_from(self.play_started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
            )
        }
    }

    fn rebase_clock(&mut self, position_ms: u64) {
        self.clock_origin_ms = position_ms;
        self.play_started_at = Instant::now();
        self.audio_ring.reset_clock(0);
    }

    fn update_clock(&mut self) -> u64 {
        let now_ms = self.clock_ms();
        let audio_empty = self.audio_backlog.is_empty() && self.audio_ring.available_read() == 0;
        if self.use_audio_clock {
            let video_stalled = self.video_queue.len() >= VIDEO_LOOKAHEAD
                && (self.video_queue.len() >= MAX_VIDEO_AHEAD_ON_AUDIO_SEARCH
                    || self
                        .video_queue
                        .back()
                        .is_some_and(|frame| frame.pts_ms() > now_ms.saturating_add(1_000)));
            if (audio_empty && (self.eof || video_stalled)) || self.info.audio_index.is_none() {
                self.audio_ring.set_paused(true);
                self.use_audio_clock = false;
                self.audio_start_ms = None;
                self.rebase_clock(now_ms);
            }
        } else if !audio_empty && self.audio_start_ms.is_some_and(|start| now_ms >= start) {
            self.rebase_clock(now_ms);
            self.use_audio_clock = true;
            self.audio_ring.set_paused(false);
        }
        now_ms
    }

    fn disable_disconnected_audio(&mut self) {
        if self.info.audio_index.is_some() && !self.audio_ring.is_consumer_active() {
            self.info.audio_index = None;
            self.decoders.audio = None;
            self.decoders.resampler = None;
            self.eof |= self.info.video_index.is_none();
            self.audio_ring.flush();
            self.audio_backlog.clear();
            self.audio_backlog_offset = 0;
        }
    }

    fn flush_audio_backlog(&mut self) {
        while self.audio_backlog_offset < self.audio_backlog.len() {
            let pushed = self
                .audio_ring
                .push(&self.audio_backlog[self.audio_backlog_offset..]);
            self.audio_backlog_offset += pushed;
            if self.audio_backlog_offset == self.audio_backlog.len() {
                self.audio_backlog.clear();
                self.audio_backlog_offset = 0;
            }
            if pushed == 0 {
                break;
            }
        }
    }

    fn reset_playback(&mut self, target_ms: u64) -> Result<(), MediaError> {
        self.audio_ring.set_paused(true);
        self.audio_ring.flush();
        self.video_queue.clear();
        self.audio_backlog.clear();
        self.audio_backlog_offset = 0;
        self.eof = self.info.video_index.is_none() && self.info.audio_index.is_none();
        self.finished = false;
        self.paused_seeked = false;
        self.audio_start_ms = None;
        self.use_audio_clock = self.info.audio_index.is_some() && self.info.video_index.is_none();
        self.seek_origin_ms = target_ms;
        self.current_pos_ms = target_ms;

        let ts = Timestamp::from_millis(i64::try_from(target_ms).unwrap_or(0));
        let result = self
            .demuxer
            .seek_to_timestamp(ts, SeekTarget::UpTo)
            .map_err(|err| MediaError::Decoder(err.to_string()))
            .and_then(|()| self.decoders.rebuild(&self.demuxer, &self.info));
        self.rebase_clock(target_ms);
        self.audio_ring
            .set_paused(self.state != PlaybackState::Playing || !self.use_audio_clock);
        result
    }

    fn handle_command(&mut self, cmd: PlayerCommand) -> bool {
        match cmd {
            PlayerCommand::Shutdown => return false,
            PlayerCommand::Play => {
                if self.state == PlaybackState::Playing {
                    return true;
                }
                let restart_at = if self.state == PlaybackState::Stopped || self.paused_seeked {
                    Some(self.current_pos_ms)
                } else if self.finished {
                    Some(0)
                } else {
                    None
                };
                if let Some(target) = restart_at {
                    if let Err(err) = self.reset_playback(target) {
                        self.sink.on_error(err.to_string());
                        return true;
                    }
                } else {
                    self.rebase_clock(self.current_pos_ms);
                }
                self.state = PlaybackState::Playing;
                self.audio_ring.set_paused(!self.use_audio_clock);
            }
            PlayerCommand::Pause => {
                self.state = PlaybackState::Paused;
                self.audio_ring.set_paused(true);
            }
            PlayerCommand::Stop => {
                self.state = PlaybackState::Stopped;
                self.audio_ring.set_paused(true);
                let _ = self.reset_playback(0);
                if self.info.video_index.is_some()
                    && let Some(frame) = self.decode_until_video_target(0)
                {
                    self.sink.on_frame(frame);
                }
            }
            PlayerCommand::Seek(target_ms) => {
                let clamped = self
                    .info
                    .probe
                    .duration_ms
                    .map_or(target_ms, |dur| target_ms.min(dur));
                if let Err(err) = self.reset_playback(clamped) {
                    self.sink.on_error(err.to_string());
                    return true;
                }
                if self.state != PlaybackState::Playing && self.info.video_index.is_some() {
                    if let Some(frame) = self.decode_until_video_target(clamped) {
                        self.sink.on_frame(frame);
                    }
                    self.paused_seeked = true;
                }
            }
            PlayerCommand::SetMuted(value) => {
                self.muted = value;
                self.audio_ring.set_muted(value);
            }
            PlayerCommand::SetLooping(value) => {
                self.looping = value;
            }
        }
        true
    }

    fn decode_until_video_target(&mut self, target_ms: u64) -> Option<Arc<VideoFrame>> {
        let video_idx = self.info.video_index?;
        let mut last_frame = None;

        while let Ok(Some(packet)) = self.demuxer.take() {
            if packet.stream_index() != video_idx {
                continue;
            }
            let Some(vdec) = self.decoders.video.as_mut() else {
                break;
            };
            if vdec.push(packet).is_err() {
                break;
            }
            while let Ok(Some(raw)) = vdec.take() {
                let pts_ms = frame_pts_ms(&raw);
                if let Some(frame) = copy_video_frame(raw, &mut self.decoders.scaler, &self.pool) {
                    last_frame = Some(frame);
                }
                if pts_ms >= target_ms {
                    return last_frame;
                }
            }
        }

        if let Some(vdec) = self.decoders.video.as_mut() {
            let _ = vdec.flush();
            while let Ok(Some(raw)) = vdec.take() {
                let pts_ms = frame_pts_ms(&raw);
                if let Some(frame) = copy_video_frame(raw, &mut self.decoders.scaler, &self.pool) {
                    last_frame = Some(frame);
                }
                if pts_ms >= target_ms {
                    break;
                }
            }
        }

        self.eof = true;
        last_frame
    }

    fn pump_packets(&mut self) -> Result<(), MediaError> {
        let min_pts_ms = self.seek_origin_ms;
        for _ in 0..128 {
            let need_video =
                self.info.video_index.is_some() && self.video_queue.len() < VIDEO_LOOKAHEAD;
            let need_audio = self.info.audio_index.is_some()
                && self.audio_backlog.is_empty()
                && self.audio_ring.available_read() < self.audio_ring.capacity() / 2
                && (self.info.video_index.is_none()
                    || self.video_queue.len() < MAX_VIDEO_AHEAD_ON_AUDIO_SEARCH);
            if !need_video && !need_audio {
                break;
            }
            match self
                .demuxer
                .take()
                .map_err(|err| MediaError::Decoder(err.to_string()))?
            {
                Some(packet) => {
                    let idx = packet.stream_index();
                    if Some(idx) == self.info.video_index
                        && let Some(vdec) = self.decoders.video.as_mut()
                    {
                        vdec.push(packet)
                            .map_err(|err| MediaError::Decoder(err.to_string()))?;
                        while let Some(raw) = vdec
                            .take()
                            .map_err(|err| MediaError::Decoder(err.to_string()))?
                        {
                            if raw.pts().as_millis().is_some_and(|ms| {
                                u64::try_from(ms).is_ok_and(|u| u + 30 < min_pts_ms)
                            }) {
                                continue;
                            }
                            if let Some(frame) =
                                copy_video_frame(raw, &mut self.decoders.scaler, &self.pool)
                            {
                                self.video_queue.push_back(frame);
                            }
                        }
                    } else if Some(idx) == self.info.audio_index
                        && let (Some(adec), Some(resampler)) = (
                            self.decoders.audio.as_mut(),
                            self.decoders.resampler.as_mut(),
                        )
                    {
                        let channels = self.decoders.channels as usize;
                        adec.push(packet)
                            .map_err(|err| MediaError::Decoder(err.to_string()))?;
                        while let Some(raw) = adec
                            .take()
                            .map_err(|err| MediaError::Decoder(err.to_string()))?
                        {
                            if raw.pts().as_millis().is_some_and(|ms| {
                                u64::try_from(ms).is_ok_and(|u| u + 30 < min_pts_ms)
                            }) {
                                continue;
                            }
                            resampler
                                .push(raw)
                                .map_err(|err| MediaError::Decoder(err.to_string()))?;
                            drain_resampled_audio(
                                resampler,
                                channels,
                                &mut self.audio_ring,
                                &mut self.audio_backlog,
                                &mut self.audio_backlog_offset,
                                &mut self.audio_start_ms,
                            )?;
                        }
                    }
                }
                None => {
                    self.eof = true;
                    if let Some(vdec) = self.decoders.video.as_mut() {
                        let _ = vdec.flush();
                        while let Ok(Some(raw)) = vdec.take() {
                            if let Some(frame) =
                                copy_video_frame(raw, &mut self.decoders.scaler, &self.pool)
                            {
                                self.video_queue.push_back(frame);
                            }
                        }
                    }
                    if let (Some(adec), Some(resampler)) = (
                        self.decoders.audio.as_mut(),
                        self.decoders.resampler.as_mut(),
                    ) {
                        let channels = self.decoders.channels as usize;
                        let _ = adec.flush();
                        while let Ok(Some(raw)) = adec.take() {
                            let _ = resampler.push(raw);
                        }
                        let _ = resampler.flush();
                        drain_resampled_audio(
                            resampler,
                            channels,
                            &mut self.audio_ring,
                            &mut self.audio_backlog,
                            &mut self.audio_backlog_offset,
                            &mut self.audio_start_ms,
                        )?;
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn run_player(
    demuxer: DemuxerWithStreamInfo<File>,
    mut info: StreamInfo,
    looping: bool,
    muted: bool,
    auto_play: bool,
    audio_config: Option<(u32, u32)>,
    audio_ring: AudioProducer,
    rx: Receiver<PlayerCommand>,
    sink: impl MediaSink,
) {
    audio_ring.set_muted(muted);
    if audio_config.is_none() {
        info.audio_index = None;
    }
    let (output_rate, output_channels) =
        audio_config.unwrap_or((FALLBACK_AUDIO_RATE, TARGET_AUDIO_CHANNELS));
    let use_audio_clock = info.audio_index.is_some() && info.video_index.is_none();
    let initial_eof = info.video_index.is_none() && info.audio_index.is_none();

    let decoders = match PlayerDecoders::new(&demuxer, &info, output_rate, output_channels) {
        Ok(dec) => dec,
        Err(err) => {
            sink.on_error(err.to_string());
            return;
        }
    };

    let state = if auto_play {
        PlaybackState::Playing
    } else {
        PlaybackState::Paused
    };
    audio_ring.set_paused(state != PlaybackState::Playing || !use_audio_clock);

    let mut ctx = PlayerContext {
        demuxer,
        info,
        decoders,
        audio_ring,
        use_audio_clock,
        audio_start_ms: None,
        paused_seeked: false,
        output_rate,
        pool: FramePool::new(FRAME_POOL_SLOTS),
        video_queue: VecDeque::with_capacity(VIDEO_LOOKAHEAD + 2),
        audio_backlog: Vec::new(),
        audio_backlog_offset: 0,
        eof: initial_eof,
        finished: false,
        state,
        muted,
        looping,
        seek_origin_ms: 0,
        clock_origin_ms: 0,
        current_pos_ms: 0,
        play_started_at: Instant::now(),
        sink,
    };

    if !auto_play && ctx.info.video_index.is_some() {
        if let Some(frame) = ctx.decode_until_video_target(0) {
            ctx.sink.on_frame(frame);
        }
        ctx.paused_seeked = true;
    }

    ctx.emit_status();
    let mut last_status_at = Instant::now();

    loop {
        while let Ok(cmd) = rx.try_recv() {
            if !ctx.handle_command(cmd) {
                return;
            }
            ctx.emit_status();
            last_status_at = Instant::now();
        }

        if ctx.state != PlaybackState::Playing {
            match rx.recv() {
                Ok(cmd) => {
                    if !ctx.handle_command(cmd) {
                        return;
                    }
                    ctx.emit_status();
                    last_status_at = Instant::now();
                }
                Err(_) => return,
            }
            continue;
        }

        ctx.disable_disconnected_audio();
        ctx.flush_audio_backlog();

        let need_video = ctx.info.video_index.is_some() && ctx.video_queue.len() < VIDEO_LOOKAHEAD;
        let need_audio = ctx.info.audio_index.is_some()
            && ctx.audio_backlog.is_empty()
            && ctx.audio_ring.available_read() < ctx.audio_ring.capacity() / 2
            && (ctx.info.video_index.is_none()
                || ctx.video_queue.len() < MAX_VIDEO_AHEAD_ON_AUDIO_SEARCH);

        if !ctx.eof
            && (need_video || need_audio)
            && let Err(err) = ctx.pump_packets()
        {
            ctx.sink.on_error(err.to_string());
            ctx.state = PlaybackState::Stopped;
            ctx.audio_ring.set_paused(true);
            ctx.emit_status();
            continue;
        }

        let clock_ms = ctx.update_clock();
        ctx.current_pos_ms = ctx
            .info
            .probe
            .duration_ms
            .map_or(clock_ms, |dur| clock_ms.min(dur));

        let mut frame_to_emit = None;
        while let Some(front) = ctx.video_queue.front() {
            if front.pts_ms() <= clock_ms.saturating_add(4) {
                frame_to_emit = ctx.video_queue.pop_front();
            } else {
                break;
            }
        }
        if let Some(frame) = frame_to_emit {
            ctx.sink.on_frame(frame);
        }

        let silent_audio_only_pending = ctx.info.video_index.is_none()
            && ctx.info.audio_index.is_none()
            && ctx.info.probe.duration_ms.is_some_and(|dur| clock_ms < dur);
        if ctx.eof
            && !silent_audio_only_pending
            && ctx.video_queue.is_empty()
            && ctx.audio_backlog.is_empty()
            && ctx.audio_ring.available_read() == 0
        {
            if ctx.looping {
                if let Err(err) = ctx.reset_playback(0) {
                    ctx.sink.on_error(err.to_string());
                    ctx.state = PlaybackState::Stopped;
                    ctx.audio_ring.set_paused(true);
                }
            } else {
                ctx.finished = true;
                ctx.state = PlaybackState::Paused;
                ctx.audio_ring.set_paused(true);
                if let Some(dur) = ctx.info.probe.duration_ms {
                    ctx.current_pos_ms = dur;
                }
                ctx.emit_status();
                last_status_at = Instant::now();
                continue;
            }
        }

        if last_status_at.elapsed() >= Duration::from_millis(100) {
            ctx.emit_status();
            last_status_at = Instant::now();
        }

        let wait_ms = ctx.video_queue.front().map_or(15, |front| {
            front.pts_ms().saturating_sub(clock_ms).clamp(1, 20)
        });
        match rx.recv_timeout(Duration::from_millis(wait_ms)) {
            Ok(cmd) => {
                if !ctx.handle_command(cmd) {
                    return;
                }
                ctx.emit_status();
                last_status_at = Instant::now();
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn drain_resampled_audio(
    resampler: &mut AudioResampler,
    channels: usize,
    audio_ring: &mut AudioProducer,
    audio_backlog: &mut Vec<f32>,
    audio_backlog_offset: &mut usize,
    audio_start_ms: &mut Option<u64>,
) -> Result<(), MediaError> {
    while let Some(resampled) = resampler
        .take()
        .map_err(|err| MediaError::Decoder(err.to_string()))?
    {
        let valid_bytes = resampled
            .samples()
            .saturating_mul(channels)
            .saturating_mul(std::mem::size_of::<f32>());
        let planes = resampled.planes();
        let plane = planes[0].data();
        let clamped = &plane[..valid_bytes.min(plane.len())];
        if !clamped.is_empty() {
            audio_start_ms.get_or_insert_with(|| {
                resampled
                    .pts()
                    .as_millis()
                    .and_then(|ms| u64::try_from(ms).ok())
                    .unwrap_or(0)
            });
        }
        push_f32_samples(clamped, audio_ring, audio_backlog, audio_backlog_offset);
    }
    Ok(())
}

fn push_f32_samples(
    bytes: &[u8],
    audio_ring: &mut AudioProducer,
    audio_backlog: &mut Vec<f32>,
    audio_backlog_offset: &mut usize,
) {
    let (head, aligned, tail) = unsafe { bytes.align_to::<f32>() };
    if head.is_empty() && tail.is_empty() {
        let pushed = audio_ring.push(aligned);
        if pushed < aligned.len() {
            if *audio_backlog_offset > 0 {
                audio_backlog.drain(..*audio_backlog_offset);
                *audio_backlog_offset = 0;
            }
            audio_backlog.extend_from_slice(&aligned[pushed..]);
        }
        return;
    }

    if *audio_backlog_offset > 0 {
        audio_backlog.drain(..*audio_backlog_offset);
        *audio_backlog_offset = 0;
    }
    let start = audio_backlog.len();
    let (chunks, _) = bytes.as_chunks::<4>();
    for chunk in chunks {
        audio_backlog.push(f32::from_ne_bytes(*chunk));
    }
    if start == 0 {
        let pushed = audio_ring.push(audio_backlog);
        *audio_backlog_offset = pushed;
        if pushed == audio_backlog.len() {
            audio_backlog.clear();
            *audio_backlog_offset = 0;
        }
    }
}

fn frame_pts_ms(frame: &FfVideoFrame) -> u64 {
    frame
        .pts()
        .as_millis()
        .and_then(|ms| u64::try_from(ms).ok())
        .unwrap_or(0)
}

fn copy_video_frame(
    raw: FfVideoFrame,
    scaler_slot: &mut Option<(FfPixelFormat, usize, usize, VideoFilter)>,
    pool: &Arc<FramePool>,
) -> Option<Arc<VideoFrame>> {
    let w = raw.width();
    let h = raw.height();
    if w == 0 || h == 0 {
        return None;
    }

    let pts_ms = frame_pts_ms(&raw);
    let src_fmt = raw.pixel_format();
    let (target_fmt, format_name) = if src_fmt.name() == "nv12" {
        (PixelFormat::Nv12, "nv12")
    } else {
        (PixelFormat::Yuv420p, "yuv420p")
    };
    if scaler_slot
        .as_ref()
        .is_none_or(|(fmt, sw, sh, _)| (*fmt, *sw, *sh) != (src_fmt, w, h))
    {
        let scaler = VideoFilter::builder(w, h, src_fmt)
            .build(&format!("scale=flags=fast_bilinear:in_color_matrix=auto:out_color_matrix=bt709:out_range=tv,format={format_name}"))
            .ok()?;
        *scaler_slot = Some((src_fmt, w, h, scaler));
    }
    let (_, _, _, scaler) = scaler_slot.as_mut()?;
    scaler.try_push(raw).ok()?;
    let frame = scaler.take().ok()??;

    let width_u32 = u32::try_from(w).ok()?;
    let height_u32 = u32::try_from(h).ok()?;
    let mut writer = pool.acquire(target_fmt, width_u32, height_u32)?;
    let (dst_y, dst_u, dst_v) = writer.planes_mut();
    let planes = frame.planes();

    copy_plane(&planes[0], w, h, dst_y);
    match target_fmt {
        PixelFormat::Nv12 => {
            let chroma_w = w.div_ceil(2) * 2;
            let chroma_h = h.div_ceil(2);
            copy_plane(&planes[1], chroma_w, chroma_h, dst_u);
        }
        PixelFormat::Yuv420p => {
            let chroma_w = w.div_ceil(2);
            let chroma_h = h.div_ceil(2);
            copy_plane(&planes[1], chroma_w, chroma_h, dst_u);
            copy_plane(&planes[2], chroma_w, chroma_h, dst_v);
        }
    }

    Some(writer.finish(pts_ms))
}

fn copy_plane(
    plane: &ac_ffmpeg::codec::video::frame::Plane<'_>,
    row_bytes: usize,
    rows: usize,
    dst: &mut [u8],
) {
    let line_size = plane.line_size();
    let src = plane.data();
    let actual_rows = rows.min(plane.line_count());
    if line_size == row_bytes
        && dst.len() >= row_bytes * actual_rows
        && src.len() >= row_bytes * actual_rows
    {
        dst[..row_bytes * actual_rows].copy_from_slice(&src[..row_bytes * actual_rows]);
        return;
    }

    for row in 0..actual_rows {
        let src_start = row * line_size;
        let dst_start = row * row_bytes;
        if src_start + row_bytes <= src.len() && dst_start + row_bytes <= dst.len() {
            dst[dst_start..dst_start + row_bytes]
                .copy_from_slice(&src[src_start..src_start + row_bytes]);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::{Duration, Instant};
    use std::{env, fs, process};

    use sdw_core::MediaKind;

    use super::{MediaPlayer, MediaSink, generate_video_thumbnail, probe_media};
    use crate::{PlaybackState, PlaybackStatus, VideoFrame, decode_image_capped};

    struct TestSink {
        frames: Arc<Mutex<Vec<(u32, u32, u64)>>>,
        statuses: Arc<Mutex<Vec<PlaybackStatus>>>,
        errors: Arc<Mutex<Vec<String>>>,
        signal: Arc<Condvar>,
    }

    impl MediaSink for TestSink {
        fn on_frame(&self, frame: Arc<VideoFrame>) {
            if let Ok(mut list) = self.frames.lock() {
                list.push((frame.width(), frame.height(), frame.pts_ms()));
            }
            self.signal.notify_all();
        }

        fn on_status(&self, status: PlaybackStatus) {
            if let Ok(mut list) = self.statuses.lock() {
                list.push(status);
            }
            self.signal.notify_all();
        }

        fn on_error(&self, message: String) {
            if let Ok(mut list) = self.errors.lock() {
                list.push(message);
            }
            self.signal.notify_all();
        }
    }

    fn wait_until<T>(
        state: &Mutex<T>,
        signal: &Condvar,
        gate: &Mutex<()>,
        timeout: Duration,
        predicate: impl Fn(&T) -> bool,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        let mut lock = gate.lock().unwrap();
        loop {
            if let Ok(guard) = state.lock()
                && predicate(&guard)
            {
                return true;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let wait_for = remaining.min(Duration::from_millis(25));
            let (guard, _) = signal.wait_timeout(lock, wait_for).unwrap();
            lock = guard;
        }
    }

    #[test]
    fn probes_thumbnails_and_plays_video_with_seek_when_ffmpeg_cli_present() {
        let dir = env::temp_dir().join(format!("sdw-media-ffmpeg-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mp4");

        let status = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=10:duration=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .arg(&video)
            .status();

        let Ok(exit) = status else {
            let _ = fs::remove_dir_all(&dir);
            return;
        };
        if !exit.success() {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let probe = probe_media(&video).unwrap();
        assert_eq!(probe.kind, MediaKind::Video);
        assert_eq!((probe.width, probe.height), (Some(160), Some(90)));
        assert!(probe.duration_ms.is_some_and(|d| d >= 800));

        let thumb = dir.join("thumb.webp");
        let thumb_probe = generate_video_thumbnail(&video, &thumb, 80).unwrap();
        assert_eq!(thumb_probe.kind, MediaKind::Video);
        let decoded = decode_image_capped(&thumb, 1000, 1000).unwrap();
        assert_eq!((decoded.width, decoded.height), (80, 45));

        let frames = Arc::new(Mutex::new(Vec::new()));
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let signal = Arc::new(Condvar::new());
        let gate = Mutex::new(());
        let sink = TestSink {
            frames: Arc::clone(&frames),
            statuses: Arc::clone(&statuses),
            errors: Arc::clone(&errors),
            signal: Arc::clone(&signal),
        };

        let timeout = Duration::from_secs(5);
        let player = MediaPlayer::open(video, false, true, false, sink).unwrap();
        assert!(wait_until(&frames, &signal, &gate, timeout, |f| !f.is_empty()));

        let before_seek_frames = frames.lock().unwrap().len();
        player.seek(500);
        assert!(wait_until(&frames, &signal, &gate, timeout, |f| f.len()
            > before_seek_frames));

        player.play();
        assert!(wait_until(&statuses, &signal, &gate, timeout, |s| {
            s.iter().any(|st| st.state == PlaybackState::Playing)
        }));
        std::thread::sleep(Duration::from_millis(120));
        player.pause();
        assert!(wait_until(&statuses, &signal, &gate, timeout, |s| {
            s.last().is_some_and(|st| st.state == PlaybackState::Paused)
        }));
        assert!(errors.lock().unwrap().is_empty());

        let before_stop_frames = frames.lock().unwrap().len();
        player.stop();
        assert!(wait_until(&statuses, &signal, &gate, timeout, |s| {
            s.last()
                .is_some_and(|st| st.state == PlaybackState::Stopped && st.position_ms == 0)
        }));
        assert!(wait_until(&frames, &signal, &gate, timeout, |f| f.len()
            > before_stop_frames));
        drop(player);

        let _ = fs::remove_dir_all(&dir);
    }

    struct SinkHarness {
        frames: Arc<Mutex<Vec<(u32, u32, u64)>>>,
        statuses: Arc<Mutex<Vec<PlaybackStatus>>>,
        errors: Arc<Mutex<Vec<String>>>,
        signal: Arc<Condvar>,
        gate: Mutex<()>,
    }

    impl SinkHarness {
        fn new() -> (Self, TestSink) {
            let frames = Arc::new(Mutex::new(Vec::new()));
            let statuses = Arc::new(Mutex::new(Vec::new()));
            let errors = Arc::new(Mutex::new(Vec::new()));
            let signal = Arc::new(Condvar::new());
            let sink = TestSink {
                frames: Arc::clone(&frames),
                statuses: Arc::clone(&statuses),
                errors: Arc::clone(&errors),
                signal: Arc::clone(&signal),
            };
            let harness = Self {
                frames,
                statuses,
                errors,
                signal,
                gate: Mutex::new(()),
            };
            (harness, sink)
        }
    }

    fn run_ffmpeg(args: &[&str], out: &std::path::Path) -> bool {
        let Ok(exit) = Command::new("ffmpeg").args(args).arg(out).status() else {
            return false;
        };
        exit.success()
    }

    #[test]
    fn normalizes_tagged_full_range_video_before_uploading_planes() {
        use super::{FramePool, copy_video_frame, decode_first_video_frame, open_demuxer};
        use crate::{ColorMatrix, ColorRange};

        let dir = env::temp_dir().join(format!("sdw-media-ffmpeg-colors-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mkv");
        if !run_ffmpeg(
            &[
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=160x90:d=0.1",
                "-vf",
                "geq=lum='if(lt(X,W/2),0,76)':cb='if(lt(X,W/2),128,85)':cr='if(lt(X,W/2),128,255)'",
                "-c:v",
                "ffv1",
                "-color_range",
                "pc",
                "-colorspace",
                "smpte170m",
            ],
            &video,
        ) {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let mut demuxer = open_demuxer(&video).unwrap();
        let raw = decode_first_video_frame(&mut demuxer, 0).unwrap().unwrap();
        let frame = copy_video_frame(raw, &mut None, &FramePool::new(1)).unwrap();
        assert_eq!(frame.color().matrix, ColorMatrix::Bt709);
        assert_eq!(frame.color().range, ColorRange::Limited);
        assert_eq!(frame.y_plane()[0], 16);
        assert!(frame.y_plane()[140].abs_diff(63) <= 2);
        assert!(frame.u_or_uv_plane()[70].abs_diff(102) <= 2);
        assert!(frame.v_plane()[70].abs_diff(240) <= 5);

        let thumb = dir.join("thumb.webp");
        generate_video_thumbnail(&video, &thumb, 80).unwrap();
        let decoded = decode_image_capped(&thumb, 1000, 1000).unwrap();
        assert_eq!(&decoded.pixels[..4], &[0, 0, 0, 255]);
        let red = &decoded.pixels[70 * 4..71 * 4];
        assert!(red[0] >= 245 && red[1] <= 5 && red[2] <= 5 && red[3] == 255);
        drop(demuxer);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn play_resumes_paused_video_with_frames_queued_at_demux_eof() {
        let dir = env::temp_dir().join(format!("sdw-media-ffmpeg-resume-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mp4");
        if !run_ffmpeg(
            &[
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=2:duration=1.5",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ],
            &video,
        ) {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let (harness, sink) = SinkHarness::new();
        let timeout = Duration::from_secs(5);
        let player = MediaPlayer::open(video, false, true, true, sink).unwrap();
        assert!(wait_until(
            &harness.frames,
            &harness.signal,
            &harness.gate,
            timeout,
            |frames| { frames.last().is_some_and(|frame| frame.2 == 500) }
        ));
        player.pause();
        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |statuses| {
                statuses
                    .last()
                    .is_some_and(|status| status.state == PlaybackState::Paused)
            }
        ));
        let (paused_pos, resume_start) = {
            let statuses = harness.statuses.lock().unwrap();
            (statuses.last().unwrap().position_ms, statuses.len())
        };
        assert!((400..1_000).contains(&paused_pos));
        player.play();
        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |statuses| { statuses.len() > resume_start }
        ));
        let resumed = harness.statuses.lock().unwrap()[resume_start];
        assert_eq!(resumed.state, PlaybackState::Playing);
        assert_eq!(resumed.position_ms, paused_pos);
        drop(player);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delayed_audio_plays_after_the_video_has_started() {
        let dir = env::temp_dir().join(format!("sdw-media-ffmpeg-delayed-audio-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mp4");
        if !run_ffmpeg(
            &[
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=30:duration=2",
                "-itsoffset",
                "1.2",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.5",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ],
            &video,
        ) {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let (harness, sink) = SinkHarness::new();
        let (tx, rx) = std::sync::mpsc::channel();
        let player =
            MediaPlayer::open_with_audio_output(video, false, false, true, sink, move |consumer| {
                tx.send(consumer).unwrap();
                Some((48_000, 2, ()))
            })
            .unwrap();
        let mut consumer = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let start = Instant::now();
        let mut first_audio_at = None;
        let mut output = [0.0; 480];
        while start.elapsed() < Duration::from_secs(5) {
            consumer.pop_into(&mut output, 2);
            if output.iter().any(|&sample| sample.abs() > 0.001) {
                first_audio_at.get_or_insert(start.elapsed());
            }
            if harness
                .statuses
                .lock()
                .unwrap()
                .last()
                .is_some_and(|status| status.state == PlaybackState::Paused)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        let first_audio_at = first_audio_at.expect("delayed track never reached the output");
        assert!(first_audio_at >= Duration::from_secs(1));
        assert!(
            harness
                .frames
                .lock()
                .unwrap()
                .iter()
                .any(|frame| frame.2 >= 1_800)
        );
        assert!(harness.errors.lock().unwrap().is_empty());
        drop(player);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replay_after_eof_and_seek_while_stopped_preserve_position() {
        let dir = env::temp_dir().join(format!("sdw-media-ffmpeg-replay-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mp4");

        if !run_ffmpeg(
            &[
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=15:duration=0.6",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.6",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ],
            &video,
        ) {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let (harness, sink) = SinkHarness::new();
        let timeout = Duration::from_secs(5);
        let player = MediaPlayer::open(video, false, true, true, sink).unwrap();

        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |s| s.last().is_some_and(|st| st.state == PlaybackState::Paused),
        ));

        let replay_start = harness.statuses.lock().unwrap().len();
        player.play();
        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |s| s[replay_start..]
                .iter()
                .any(|st| st.state == PlaybackState::Playing),
        ));
        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |s| s[replay_start..]
                .iter()
                .any(|st| st.state == PlaybackState::Paused),
        ));

        player.stop();
        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |s| s
                .last()
                .is_some_and(|st| st.state == PlaybackState::Stopped),
        ));

        let before_seek_frames = harness.frames.lock().unwrap().len();
        player.seek(350);
        assert!(wait_until(
            &harness.frames,
            &harness.signal,
            &harness.gate,
            timeout,
            |f| f.len() > before_seek_frames,
        ));

        let play_after_seek = harness.statuses.lock().unwrap().len();
        player.play();
        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |s| s[play_after_seek..]
                .iter()
                .any(|st| st.state == PlaybackState::Playing),
        ));
        let statuses = harness.statuses.lock().unwrap();
        let first_playing = statuses[play_after_seek..]
            .iter()
            .find(|st| st.state == PlaybackState::Playing)
            .unwrap();
        assert!(first_playing.position_ms >= 300);
        assert!(harness.errors.lock().unwrap().is_empty());
        drop(statuses);
        drop(player);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn video_and_audio_only_finish_without_audio_output() {
        let dir = env::temp_dir().join(format!("sdw-media-ffmpeg-noaudio-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mp4");
        let audio = dir.join("clip.m4a");

        if !run_ffmpeg(
            &[
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=15:duration=0.5",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.5",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ],
            &video,
        ) || !run_ffmpeg(
            &[
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.4",
                "-c:a",
                "aac",
            ],
            &audio,
        ) {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let timeout = Duration::from_secs(5);
        let (v_harness, v_sink) = SinkHarness::new();
        let v_player =
            MediaPlayer::open_with_audio_output(video, false, true, true, v_sink, |_| {
                None::<(_, _, ())>
            })
            .unwrap();
        assert!(wait_until(
            &v_harness.statuses,
            &v_harness.signal,
            &v_harness.gate,
            timeout,
            |s| s.last().is_some_and(|st| st.state == PlaybackState::Paused),
        ));
        assert!(!v_harness.frames.lock().unwrap().is_empty());
        assert!(v_harness.errors.lock().unwrap().is_empty());
        drop(v_player);

        let (a_harness, a_sink) = SinkHarness::new();
        let a_player =
            MediaPlayer::open_with_audio_output(audio, false, true, true, a_sink, |_| {
                None::<(_, _, ())>
            })
            .unwrap();
        assert!(wait_until(
            &a_harness.statuses,
            &a_harness.signal,
            &a_harness.gate,
            timeout,
            |s| s.last().is_some_and(|st| st.state == PlaybackState::Paused),
        ));
        assert!(a_harness.errors.lock().unwrap().is_empty());
        drop(a_player);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn video_continues_playing_after_shorter_audio_track_ends() {
        let dir = env::temp_dir().join(format!("sdw-media-ffmpeg-short-audio-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let video = dir.join("clip.mp4");

        if !run_ffmpeg(
            &[
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x90:rate=20:duration=1.2",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ],
            &video,
        ) {
            let _ = fs::remove_dir_all(&dir);
            return;
        }

        let (harness, sink) = SinkHarness::new();
        let timeout = Duration::from_secs(5);
        let player = MediaPlayer::open(video, false, true, true, sink).unwrap();

        assert!(wait_until(
            &harness.statuses,
            &harness.signal,
            &harness.gate,
            timeout,
            |s| s.last().is_some_and(|st| st.state == PlaybackState::Paused),
        ));
        assert!(
            harness
                .frames
                .lock()
                .unwrap()
                .iter()
                .any(|(_, _, pts)| *pts >= 800)
        );
        assert!(harness.errors.lock().unwrap().is_empty());
        drop(player);

        let _ = fs::remove_dir_all(&dir);
    }
}
