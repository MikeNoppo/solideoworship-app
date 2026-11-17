use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ac_ffmpeg::codec::audio::{AudioDecoder, AudioResampler, ChannelLayout, SampleFormat};
use ac_ffmpeg::codec::video::frame::get_pixel_format;
use ac_ffmpeg::codec::video::scaler::Algorithm;
use ac_ffmpeg::codec::video::{
    PixelFormat as FfPixelFormat, VideoDecoder, VideoFrame as FfVideoFrame, VideoFrameScaler,
};
use ac_ffmpeg::codec::{AudioCodecParameters, Decoder};
use ac_ffmpeg::format::demuxer::{Demuxer, DemuxerWithStreamInfo, SeekTarget};
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::time::Timestamp;
use image::ImageEncoder;
use image::codecs::webp::WebPEncoder;
use sdw_core::MediaKind;

use crate::MediaError;
use crate::audio::{AudioOutput, AudioRingBuffer};
use crate::image::{fit_within, temp_thumb_path};
use crate::video::{FramePool, PixelFormat, PlaybackState, PlaybackStatus, VideoFrame};

const AUDIO_RING_SAMPLES: usize = 65_536;
const TARGET_AUDIO_CHANNELS: u32 = 2;
const FALLBACK_AUDIO_RATE: u32 = 48_000;
const FRAME_POOL_SLOTS: usize = 5;
const VIDEO_LOOKAHEAD: usize = 3;

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
    let rgba_fmt = get_pixel_format("rgba");
    let mut scaler = VideoFrameScaler::builder()
        .source_pixel_format(frame.pixel_format())
        .source_width(frame.width())
        .source_height(frame.height())
        .target_pixel_format(rgba_fmt)
        .target_width(dst_w as usize)
        .target_height(dst_h as usize)
        .algorithm(Algorithm::Bilinear)
        .build()
        .map_err(|err| MediaError::Decoder(err.to_string()))?;

    let scaled = scaler
        .scale(&frame)
        .map_err(|err| MediaError::Decoder(err.to_string()))?;
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
    worker: Option<JoinHandle<()>>,
    probe: MediaProbe,
}

impl MediaPlayer {
    pub fn open(
        path: PathBuf,
        looping: bool,
        muted: bool,
        auto_play: bool,
        sink: impl MediaSink,
    ) -> Result<Self, MediaError> {
        let demuxer = open_demuxer(&path)?;
        let info = inspect_streams(&demuxer);
        let probe = info.probe;

        let (tx, rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("sdw-media-player".to_owned())
            .spawn(move || {
                run_player(demuxer, info, looping, muted, auto_play, rx, sink);
            })?;

        Ok(Self {
            tx,
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
}

impl Drop for MediaPlayer {
    fn drop(&mut self) {
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
    scaler: Option<(FfPixelFormat, usize, usize, VideoFrameScaler)>,
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
    audio_ring: Arc<AudioRingBuffer>,
    _audio_output: Option<AudioOutput>,
    use_audio_clock: bool,
    paused_seeked: bool,
    output_rate: u32,
    pool: Arc<FramePool>,
    nv12_fmt: FfPixelFormat,
    yuv420p_fmt: FfPixelFormat,
    video_queue: VecDeque<Arc<VideoFrame>>,
    audio_backlog: Vec<f32>,
    audio_backlog_offset: usize,
    eof: bool,
    state: PlaybackState,
    muted: bool,
    looping: bool,
    seek_origin_ms: u64,
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
            self.seek_origin_ms.saturating_add(
                self.audio_ring.played_frames().saturating_mul(1_000)
                    / u64::from(self.output_rate.max(1)),
            )
        } else {
            self.seek_origin_ms.saturating_add(
                u64::try_from(self.play_started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
            )
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
        self.audio_ring.flush();
        self.audio_ring.reset_clock(0);
        self.video_queue.clear();
        self.audio_backlog.clear();
        self.audio_backlog_offset = 0;
        self.eof = false;
        self.paused_seeked = false;
        self.use_audio_clock = self._audio_output.is_some();
        self.seek_origin_ms = target_ms;
        self.current_pos_ms = target_ms;
        self.play_started_at = Instant::now();

        let ts = Timestamp::from_millis(i64::try_from(target_ms).unwrap_or(0));
        self.demuxer
            .seek_to_timestamp(ts, SeekTarget::UpTo)
            .map_err(|err| MediaError::Decoder(err.to_string()))?;
        self.decoders.rebuild(&self.demuxer, &self.info)?;
        Ok(())
    }

    fn handle_command(&mut self, cmd: PlayerCommand) -> bool {
        match cmd {
            PlayerCommand::Shutdown => return false,
            PlayerCommand::Play => {
                if self.state == PlaybackState::Stopped {
                    let _ = self.reset_playback(0);
                } else if self.paused_seeked {
                    let _ = self.reset_playback(self.current_pos_ms);
                } else {
                    self.seek_origin_ms = self.current_pos_ms;
                    self.audio_ring.reset_clock(0);
                    self.play_started_at = Instant::now();
                }
                self.state = PlaybackState::Playing;
                self.audio_ring.set_paused(false);
            }
            PlayerCommand::Pause => {
                self.state = PlaybackState::Paused;
                self.audio_ring.set_paused(true);
                self.seek_origin_ms = self.current_pos_ms;
                self.audio_ring.reset_clock(0);
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
                if let Some(frame) = copy_video_frame(
                    &raw,
                    &mut self.decoders.scaler,
                    &self.pool,
                    self.nv12_fmt,
                    self.yuv420p_fmt,
                ) {
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
                if let Some(frame) = copy_video_frame(
                    &raw,
                    &mut self.decoders.scaler,
                    &self.pool,
                    self.nv12_fmt,
                    self.yuv420p_fmt,
                ) {
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
                && self.audio_ring.available_read() < self.audio_ring.capacity() / 2;
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
                            if let Some(frame) = copy_video_frame(
                                &raw,
                                &mut self.decoders.scaler,
                                &self.pool,
                                self.nv12_fmt,
                                self.yuv420p_fmt,
                            ) {
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
                                &self.audio_ring,
                                &mut self.audio_backlog,
                                &mut self.audio_backlog_offset,
                            )?;
                        }
                    }
                }
                None => {
                    self.eof = true;
                    if let Some(vdec) = self.decoders.video.as_mut() {
                        let _ = vdec.flush();
                        while let Ok(Some(raw)) = vdec.take() {
                            if let Some(frame) = copy_video_frame(
                                &raw,
                                &mut self.decoders.scaler,
                                &self.pool,
                                self.nv12_fmt,
                                self.yuv420p_fmt,
                            ) {
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
                            &self.audio_ring,
                            &mut self.audio_backlog,
                            &mut self.audio_backlog_offset,
                        )?;
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}

fn run_player(
    demuxer: DemuxerWithStreamInfo<File>,
    info: StreamInfo,
    looping: bool,
    muted: bool,
    auto_play: bool,
    rx: Receiver<PlayerCommand>,
    sink: impl MediaSink,
) {
    let audio_ring = AudioRingBuffer::new(AUDIO_RING_SAMPLES);
    audio_ring.set_muted(muted);
    let audio_output = info
        .audio_index
        .and_then(|_| AudioOutput::open(Arc::clone(&audio_ring)));
    let output_rate = audio_output
        .as_ref()
        .map_or(FALLBACK_AUDIO_RATE, |out| out.sample_rate);
    let output_channels = audio_output
        .as_ref()
        .map_or(TARGET_AUDIO_CHANNELS, |out| u32::from(out.channels));
    let use_audio_clock = audio_output.is_some();

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
    audio_ring.set_paused(state != PlaybackState::Playing);

    let mut ctx = PlayerContext {
        demuxer,
        info,
        decoders,
        audio_ring,
        _audio_output: audio_output,
        use_audio_clock,
        paused_seeked: false,
        output_rate,
        pool: FramePool::new(FRAME_POOL_SLOTS),
        nv12_fmt: get_pixel_format("nv12"),
        yuv420p_fmt: get_pixel_format("yuv420p"),
        video_queue: VecDeque::with_capacity(VIDEO_LOOKAHEAD + 2),
        audio_backlog: Vec::new(),
        audio_backlog_offset: 0,
        eof: false,
        state,
        muted,
        looping,
        seek_origin_ms: 0,
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

        ctx.flush_audio_backlog();

        let need_video = ctx.info.video_index.is_some() && ctx.video_queue.len() < VIDEO_LOOKAHEAD;
        let need_audio = ctx.info.audio_index.is_some()
            && ctx.audio_backlog.is_empty()
            && ctx.audio_ring.available_read() < ctx.audio_ring.capacity() / 2;

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

        if ctx.eof
            && ctx.use_audio_clock
            && ctx.audio_backlog.is_empty()
            && ctx.audio_ring.available_read() == 0
        {
            ctx.seek_origin_ms = ctx.clock_ms();
            ctx.play_started_at = Instant::now();
            ctx.use_audio_clock = false;
        }

        let clock_ms = ctx.clock_ms();
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

        if ctx.eof
            && ctx.video_queue.is_empty()
            && ctx.audio_backlog.is_empty()
            && (!ctx.use_audio_clock || ctx.audio_ring.available_read() == 0)
        {
            if ctx.looping {
                if let Err(err) = ctx.reset_playback(0) {
                    ctx.sink.on_error(err.to_string());
                    ctx.state = PlaybackState::Stopped;
                    ctx.audio_ring.set_paused(true);
                }
            } else {
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
    audio_ring: &AudioRingBuffer,
    audio_backlog: &mut Vec<f32>,
    audio_backlog_offset: &mut usize,
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
        push_f32_samples(clamped, audio_ring, audio_backlog, audio_backlog_offset);
    }
    Ok(())
}

fn push_f32_samples(
    bytes: &[u8],
    audio_ring: &AudioRingBuffer,
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
    raw: &FfVideoFrame,
    scaler_slot: &mut Option<(FfPixelFormat, usize, usize, VideoFrameScaler)>,
    pool: &Arc<FramePool>,
    nv12_fmt: FfPixelFormat,
    yuv420p_fmt: FfPixelFormat,
) -> Option<Arc<VideoFrame>> {
    let w = raw.width();
    let h = raw.height();
    if w == 0 || h == 0 {
        return None;
    }

    let pts_ms = frame_pts_ms(raw);
    let src_fmt = raw.pixel_format();

    let converted_holder;
    let (frame_ref, target_fmt) = if src_fmt == nv12_fmt {
        (raw, PixelFormat::Nv12)
    } else if src_fmt == yuv420p_fmt {
        (raw, PixelFormat::Yuv420p)
    } else {
        let needs_rebuild = scaler_slot
            .as_ref()
            .is_none_or(|(fmt, sw, sh, _)| *fmt != src_fmt || *sw != w || *sh != h);
        if needs_rebuild {
            let scaler = VideoFrameScaler::builder()
                .source_pixel_format(src_fmt)
                .source_width(w)
                .source_height(h)
                .target_pixel_format(yuv420p_fmt)
                .target_width(w)
                .target_height(h)
                .algorithm(Algorithm::FastBilinear)
                .build()
                .ok()?;
            *scaler_slot = Some((src_fmt, w, h, scaler));
        }
        let (_, _, _, scaler) = scaler_slot.as_mut()?;
        converted_holder = scaler.scale(raw).ok()?;
        (&converted_holder, PixelFormat::Yuv420p)
    };

    let width_u32 = u32::try_from(w).ok()?;
    let height_u32 = u32::try_from(h).ok()?;
    let mut writer = pool.acquire(target_fmt, width_u32, height_u32)?;
    let (dst_y, dst_u, dst_v) = writer.planes_mut();
    let planes = frame_ref.planes();

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
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::{env, fs, process};

    use sdw_core::MediaKind;

    use super::{MediaPlayer, MediaSink, generate_video_thumbnail, probe_media};
    use crate::{PlaybackState, PlaybackStatus, VideoFrame, decode_image_capped};

    struct TestSink {
        frames: Arc<Mutex<Vec<(u32, u32, u64)>>>,
        statuses: Arc<Mutex<Vec<PlaybackStatus>>>,
        errors: Arc<Mutex<Vec<String>>>,
    }

    impl MediaSink for TestSink {
        fn on_frame(&self, frame: Arc<VideoFrame>) {
            if let Ok(mut list) = self.frames.lock() {
                list.push((frame.width(), frame.height(), frame.pts_ms()));
            }
        }

        fn on_status(&self, status: PlaybackStatus) {
            if let Ok(mut list) = self.statuses.lock() {
                list.push(status);
            }
        }

        fn on_error(&self, message: String) {
            if let Ok(mut list) = self.errors.lock() {
                list.push(message);
            }
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
        let sink = TestSink {
            frames: Arc::clone(&frames),
            statuses: Arc::clone(&statuses),
            errors: Arc::clone(&errors),
        };

        let player = MediaPlayer::open(video, false, true, false, sink).unwrap();
        std::thread::sleep(Duration::from_millis(120));
        assert!(!frames.lock().unwrap().is_empty());

        player.seek(500);
        std::thread::sleep(Duration::from_millis(120));
        player.play();
        std::thread::sleep(Duration::from_millis(250));
        player.pause();
        std::thread::sleep(Duration::from_millis(50));

        assert!(errors.lock().unwrap().is_empty());
        let recorded_statuses = statuses.lock().unwrap();
        assert!(
            recorded_statuses
                .iter()
                .any(|s| s.state == PlaybackState::Playing)
        );
        assert_eq!(
            recorded_statuses.last().unwrap().state,
            PlaybackState::Paused
        );
        drop(recorded_statuses);

        let before_stop_frames = frames.lock().unwrap().len();
        player.stop();
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(
            statuses.lock().unwrap().last().unwrap().state,
            PlaybackState::Stopped
        );
        assert_eq!(statuses.lock().unwrap().last().unwrap().position_ms, 0);
        assert!(frames.lock().unwrap().len() > before_stop_frames);
        drop(player);

        let _ = fs::remove_dir_all(&dir);
    }
}
