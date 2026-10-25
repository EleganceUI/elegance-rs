//! Audio playback, audio and camera capture, and video.
//!
//! Players, recorders, and streams are cheap handles: clones refer to the same one. Their
//! reactive reads (`is_playing`, `time`, ...) are for the UI thread, like any state.

use std::path::Path;
use std::ptr::NonNull;
use std::sync::Arc;

use elegance_sys as sys;

use crate::dsl::ElgNew;
use crate::error::{self, ffi, Error};
use crate::ffi as cffi;
use crate::geometry::{ContentMode, Size};
use crate::reactive::Binding;
use crate::view::{IntoView, View};

macro_rules! handle {
    ($name:ident, $raw:ident, $clone:ident, $free:ident) => {
        impl Clone for $name {
            fn clone(&self) -> Self {
                Self {
                    ptr: NonNull::new(unsafe { sys::$clone(self.ptr.as_ptr()) }).unwrap(),
                }
            }
        }
        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { sys::$free(self.ptr.as_ptr()) }
            }
        }
        impl $name {
            fn from_raw(ptr: *mut sys::$raw) -> Result<Self, Error> {
                error::result()?;
                Ok(Self {
                    ptr: NonNull::new(ptr)
                        .ok_or_else(|| Error::new(crate::ErrorKind::Other, "no handle"))?,
                })
            }
            fn as_ptr(&self) -> *const sys::$raw {
                self.ptr.as_ptr()
            }
        }
    };
}

fn error_string(f: impl FnOnce(sys::ElgStrSink, *mut std::ffi::c_void)) -> Option<String> {
    let message = cffi::read_string(f);
    crate::error::check();
    (!message.is_empty()).then_some(message)
}

/// An audio device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AudioDevice {
    /// For `AudioStream`/`AudioRecorder` device ids.
    pub id: String,
    /// As the system shows it.
    pub name: String,
    pub is_default: bool,
}

fn devices(list: unsafe extern "C" fn(sys::ElgClosure)) -> Vec<AudioDevice> {
    let mut result = Vec::new();
    let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
        let d = &*(args as *const sys::ElgDevice);
        result.push(AudioDevice {
            id: d.id.as_str().to_owned(),
            name: d.name.as_str().to_owned(),
            is_default: d.is_default,
        });
    };
    ffi!(list(cffi::borrowed(&mut each)));
    result
}

pub fn audio_output_devices() -> Vec<AudioDevice> {
    devices(sys::elg_audio_output_devices)
}

pub fn audio_input_devices() -> Vec<AudioDevice> {
    devices(sys::elg_audio_input_devices)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayOptions {
    /// 0...1 (and beyond, amplified).
    pub volume: f32,
    /// Playback speed and pitch: 2 is an octave up.
    pub pitch: f32,
    /// -1 left ... 1 right.
    pub pan: f32,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            pan: 0.0,
        }
    }
}

/// A short sound, decoded into memory, played any number of times at once (UI clicks, game
/// effects).
pub struct Sound {
    ptr: NonNull<sys::Sound>,
}
handle!(Sound, Sound, elg_sound_clone, elg_sound_free);
unsafe impl Send for Sound {}
unsafe impl Sync for Sound {}

impl Sound {
    pub fn load(file: impl AsRef<Path>) -> Result<Sound, Error> {
        let file = cffi::path_str(file.as_ref());
        Self::from_raw(unsafe { sys::elg_sound_load_file(cffi::s(&file)) })
    }
    /// Encoded audio in memory (copied).
    pub fn load_data(data: &[u8]) -> Result<Sound, Error> {
        Self::from_raw(unsafe { sys::elg_sound_load_data(sys::ElgBytes::new(data)) })
    }
    pub fn play(&self) {
        self.play_with(PlayOptions::default())
    }
    pub fn play_with(&self, options: PlayOptions) {
        ffi!(sys::elg_sound_play(
            self.as_ptr(),
            options.volume,
            options.pitch,
            options.pan
        ));
    }
    pub fn duration(&self) -> f64 {
        unsafe { sys::elg_sound_duration(self.as_ptr()) }
    }
}

/// Streams a music file (MP3, FLAC, WAV, OGG): play, pause, seek, loop.
pub struct AudioPlayer {
    ptr: NonNull<sys::AudioPlayer>,
}
handle!(
    AudioPlayer,
    AudioPlayer,
    elg_audio_player_clone,
    elg_audio_player_free
);

#[elegance_macros::elg_builder]
impl AudioPlayer {
    pub fn new(file: impl AsRef<Path>) -> Result<AudioPlayer, Error> {
        let file = cffi::path_str(file.as_ref());
        Self::from_raw(unsafe { sys::elg_audio_player_new(cffi::s(&file)) })
    }
    pub fn play(&self) {
        ffi!(sys::elg_audio_player_control(self.as_ptr(), 0));
    }
    pub fn pause(&self) {
        ffi!(sys::elg_audio_player_control(self.as_ptr(), 1));
    }
    pub fn toggle(&self) {
        ffi!(sys::elg_audio_player_control(self.as_ptr(), 2));
    }
    /// Pauses and goes back to the start.
    pub fn stop(&self) {
        ffi!(sys::elg_audio_player_control(self.as_ptr(), 3));
    }
    pub fn seek(&self, seconds: f64) {
        ffi!(sys::elg_audio_player_seek(self.as_ptr(), seconds));
    }
    pub fn set_volume(&self, volume: f32) {
        ffi!(sys::elg_audio_player_set_volume(self.as_ptr(), volume));
    }
    pub fn set_looping(&self, looping: bool) {
        ffi!(sys::elg_audio_player_set_looping(self.as_ptr(), looping));
    }
    pub fn is_playing(&self) -> bool {
        ffi!(sys::elg_audio_player_flag(self.as_ptr(), 0))
    }
    pub fn is_looping(&self) -> bool {
        ffi!(sys::elg_audio_player_flag(self.as_ptr(), 1))
    }
    /// Seconds from the start.
    pub fn time(&self) -> f64 {
        ffi!(sys::elg_audio_player_value(self.as_ptr(), 0))
    }
    pub fn duration(&self) -> f64 {
        ffi!(sys::elg_audio_player_value(self.as_ptr(), 1))
    }
    pub fn volume(&self) -> f32 {
        ffi!(sys::elg_audio_player_value(self.as_ptr(), 2)) as f32
    }
    /// The playback position for a Slider: `Slider(player.time_binding(), max: duration)`.
    pub fn time_binding(&self) -> Binding<f32> {
        Binding::from_cpp(ffi!(sys::elg_audio_player_binding(self.as_ptr(), 0)))
    }
    pub fn volume_binding(&self) -> Binding<f32> {
        Binding::from_cpp(ffi!(sys::elg_audio_player_binding(self.as_ptr(), 1)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioStreamOptions {
    /// Empty: the system's default output (followed when it changes).
    pub device_id: String,
    pub sample_rate: u32,
    pub channels: u32,
}

impl Default for AudioStreamOptions {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            sample_rate: 48000,
            channels: 2,
        }
    }
}

/// Plays samples you produce: queue them with `write` from any thread, or mix on demand on the
/// audio thread with `AudioStream::with_render`.
pub struct AudioStream {
    ptr: NonNull<sys::AudioStream>,
}
handle!(
    AudioStream,
    AudioStream,
    elg_audio_stream_clone,
    elg_audio_stream_free
);
unsafe impl Send for AudioStream {}
unsafe impl Sync for AudioStream {}

#[elegance_macros::elg_builder]
impl AudioStream {
    pub fn new(options: AudioStreamOptions) -> Result<AudioStream, Error> {
        Self::make(options, sys::ElgClosure::NONE)
    }
    /// `render` fills each buffer of interleaved samples. It runs on the audio thread: keep it
    /// short and lock-light.
    pub fn with_render(
        options: AudioStreamOptions,
        render: impl Fn(&mut [f32]) + Send + 'static,
    ) -> Result<AudioStream, Error> {
        let render = cffi::send_closure(move |args, _| unsafe {
            let buffer = &*(args as *const sys::ElgRenderBuffer);
            render(std::slice::from_raw_parts_mut(buffer.samples, buffer.count));
        });
        Self::make(options, render)
    }
    fn make(options: AudioStreamOptions, render: sys::ElgClosure) -> Result<AudioStream, Error> {
        Self::from_raw(unsafe {
            sys::elg_audio_stream_new(
                cffi::s(&options.device_id),
                options.sample_rate,
                options.channels,
                render,
            )
        })
    }
    /// Queues interleaved samples. Any thread.
    pub fn write(&self, samples: &[f32]) {
        ffi!(sys::elg_audio_stream_write(
            self.as_ptr(),
            samples.as_ptr(),
            samples.len()
        ));
    }
    /// Drops what's queued.
    pub fn clear(&self) {
        ffi!(sys::elg_audio_stream_control(self.as_ptr(), 0));
    }
    pub fn start(&self) {
        ffi!(sys::elg_audio_stream_control(self.as_ptr(), 1));
    }
    pub fn stop(&self) {
        ffi!(sys::elg_audio_stream_control(self.as_ptr(), 2));
    }
    pub fn queued_frames(&self) -> usize {
        ffi!(sys::elg_audio_stream_queued_frames(self.as_ptr()))
    }
    pub fn set_volume(&self, volume: f32) {
        ffi!(sys::elg_audio_stream_set_volume(self.as_ptr(), volume));
    }
    /// Why it can't play, if it can't.
    pub fn error(&self) -> Option<String> {
        error_string(|sink, ctx| unsafe { sys::elg_audio_stream_error(self.as_ptr(), sink, ctx) })
    }
    pub fn sample_rate(&self) -> u32 {
        unsafe { sys::elg_audio_stream_sample_rate(self.as_ptr()) }
    }
    pub fn channels(&self) -> u32 {
        unsafe { sys::elg_audio_stream_channels(self.as_ptr()) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioRecorderOptions {
    /// Empty: the system's default input.
    pub device_id: String,
    pub sample_rate: u32,
    pub channels: u32,
    /// false: only `on_samples` sees the audio (for streaming).
    pub keeps_samples: bool,
}

impl Default for AudioRecorderOptions {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            sample_rate: 48000,
            channels: 1,
            keeps_samples: true,
        }
    }
}

/// Records from a microphone. If access is denied, `error()` says so. Make it on the UI thread.
pub struct AudioRecorder {
    ptr: NonNull<sys::AudioRecorder>,
}
handle!(
    AudioRecorder,
    AudioRecorder,
    elg_audio_recorder_clone,
    elg_audio_recorder_free
);

#[elegance_macros::elg_builder]
impl AudioRecorder {
    pub fn new(options: AudioRecorderOptions) -> Result<AudioRecorder, Error> {
        Self::make(options, sys::ElgClosure::NONE)
    }
    /// `on_samples` sees each block of interleaved samples on the audio thread: keep it short,
    /// and don't touch views or state from it.
    pub fn with_samples(
        options: AudioRecorderOptions,
        on_samples: impl Fn(&[f32]) + Send + 'static,
    ) -> Result<AudioRecorder, Error> {
        let on_samples = cffi::send_closure(move |args, _| unsafe {
            let samples = &*(args as *const sys::ElgSamples);
            on_samples(std::slice::from_raw_parts(samples.samples, samples.count));
        });
        Self::make(options, on_samples)
    }
    fn make(
        options: AudioRecorderOptions,
        on_samples: sys::ElgClosure,
    ) -> Result<AudioRecorder, Error> {
        Self::from_raw(unsafe {
            sys::elg_audio_recorder_new(
                cffi::s(&options.device_id),
                options.sample_rate,
                options.channels,
                on_samples,
                options.keeps_samples,
            )
        })
    }
    /// Continues the recording.
    pub fn start(&self) {
        ffi!(sys::elg_audio_recorder_control(self.as_ptr(), 0));
    }
    pub fn stop(&self) {
        ffi!(sys::elg_audio_recorder_control(self.as_ptr(), 1));
    }
    pub fn toggle(&self) {
        ffi!(sys::elg_audio_recorder_control(self.as_ptr(), 2));
    }
    /// Drops what was recorded.
    pub fn clear(&self) {
        ffi!(sys::elg_audio_recorder_control(self.as_ptr(), 3));
    }
    pub fn is_recording(&self) -> bool {
        ffi!(sys::elg_audio_recorder_is_recording(self.as_ptr()))
    }
    /// Seconds recorded.
    pub fn duration(&self) -> f64 {
        ffi!(sys::elg_audio_recorder_duration(self.as_ptr()))
    }
    /// The input's recent peak, 0...1, while recording.
    pub fn level(&self) -> f32 {
        ffi!(sys::elg_audio_recorder_level(self.as_ptr()))
    }
    pub fn error(&self) -> Option<String> {
        error_string(|sink, ctx| unsafe { sys::elg_audio_recorder_error(self.as_ptr(), sink, ctx) })
    }
    pub fn sample_rate(&self) -> u32 {
        unsafe { sys::elg_audio_recorder_sample_rate(self.as_ptr()) }
    }
    pub fn channels(&self) -> u32 {
        unsafe { sys::elg_audio_recorder_channels(self.as_ptr()) }
    }
    /// The recording, interleaved.
    pub fn samples(&self) -> Vec<f32> {
        let mut result = Vec::new();
        let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
            let s = &*(args as *const sys::ElgSamples);
            result.extend_from_slice(std::slice::from_raw_parts(s.samples, s.count));
        };
        ffi!(sys::elg_audio_recorder_samples(
            self.as_ptr(),
            cffi::borrowed(&mut each)
        ));
        result
    }
    pub fn save_wav(&self, file: impl AsRef<Path>) -> Result<(), Error> {
        let file = cffi::path_str(file.as_ref());
        unsafe { sys::elg_audio_recorder_save_wav(self.as_ptr(), cffi::s(&file)) };
        error::result()
    }
    /// The recording as a WAV file's bytes.
    pub fn wav(&self) -> Vec<u8> {
        let mut result = Vec::new();
        let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
            result.extend_from_slice((*(args as *const sys::ElgBytes)).as_slice());
        };
        ffi!(sys::elg_audio_recorder_wav(
            self.as_ptr(),
            cffi::borrowed(&mut each)
        ));
        result
    }
    /// The recording as a Sound, to play back.
    pub fn sound(&self) -> Result<Sound, Error> {
        Sound::from_raw(unsafe { sys::elg_audio_recorder_sound(self.as_ptr()) })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CameraDevice {
    pub id: String,
    pub name: String,
}

pub fn camera_devices() -> Vec<CameraDevice> {
    devices(sys::elg_camera_devices)
        .into_iter()
        .map(|d| CameraDevice {
            id: d.id,
            name: d.name,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraOptions {
    /// Empty: the first camera.
    pub device_id: String,
    pub width: u32,
    pub height: u32,
    /// The preview shows a mirror image, as people expect of themselves.
    pub mirrored: bool,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            width: 1280,
            height: 720,
            mirrored: true,
        }
    }
}

/// A camera feed for `CameraView`, and snapshots. Make it on the UI thread.
pub struct Camera {
    ptr: NonNull<sys::Camera>,
}
handle!(Camera, Camera, elg_camera_clone, elg_camera_free);

impl Camera {
    pub fn new(options: CameraOptions) -> Result<Camera, Error> {
        Self::from_raw(unsafe {
            sys::elg_camera_new(
                cffi::s(&options.device_id),
                options.width,
                options.height,
                options.mirrored,
            )
        })
    }
    pub fn start(&self) {
        ffi!(sys::elg_camera_start(self.as_ptr(), true));
    }
    pub fn stop(&self) {
        ffi!(sys::elg_camera_start(self.as_ptr(), false));
    }
    pub fn is_running(&self) -> bool {
        ffi!(sys::elg_camera_is_running(self.as_ptr()))
    }
    /// Pixels of the frames; zero until the first one.
    pub fn size(&self) -> Size {
        Size::from_c(ffi!(sys::elg_camera_size(self.as_ptr())))
    }
    pub fn error(&self) -> Option<String> {
        error_string(|sink, ctx| unsafe { sys::elg_camera_error(self.as_ptr(), sink, ctx) })
    }
    pub fn mirrored(&self) -> bool {
        unsafe { sys::elg_camera_mirrored(self.as_ptr()) }
    }
    /// The latest frame as PNG bytes, if there's one.
    pub fn snapshot(&self) -> Option<Vec<u8>> {
        let mut result = Vec::new();
        let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
            result.extend_from_slice((*(args as *const sys::ElgBytes)).as_slice());
        };
        ffi!(sys::elg_camera_snapshot(
            self.as_ptr(),
            cffi::borrowed(&mut each)
        ))
        .then_some(result)
    }
}

/// Shows a camera's feed.
pub struct CameraView {
    camera: Camera,
    mode: ContentMode,
}

#[elegance_macros::elg_builder]
impl CameraView {
    pub fn new(camera: &Camera) -> Self {
        Self {
            camera: camera.clone(),
            mode: ContentMode::Fill,
        }
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
}

impl ElgNew<(&Camera,)> for CameraView {
    type Out = CameraView;
    fn elg_new((camera,): (&Camera,)) -> Self {
        CameraView::new(camera)
    }
}

impl ElgNew<(Camera,)> for CameraView {
    type Out = CameraView;
    fn elg_new((camera,): (Camera,)) -> Self {
        CameraView {
            camera,
            mode: ContentMode::Fill,
        }
    }
}

impl IntoView for CameraView {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::elg_camera_view(
            self.camera.as_ptr(),
            self.mode as i32
        )))
    }
}

/// Plays a video file or URL (with sound), shown by `VideoView`.
pub struct VideoPlayer {
    ptr: NonNull<sys::VideoPlayer>,
}
handle!(
    VideoPlayer,
    VideoPlayer,
    elg_video_player_clone,
    elg_video_player_free
);

#[elegance_macros::elg_builder]
impl VideoPlayer {
    /// A file path or URL.
    pub fn new(source: &str) -> Result<VideoPlayer, Error> {
        Self::from_raw(unsafe { sys::elg_video_player_new(cffi::s(source)) })
    }
    pub fn file(file: impl AsRef<Path>) -> Result<VideoPlayer, Error> {
        let file = cffi::path_str(file.as_ref());
        Self::from_raw(unsafe { sys::elg_video_player_new_file(cffi::s(&file)) })
    }
    pub fn play(&self) {
        ffi!(sys::elg_video_player_control(self.as_ptr(), 0));
    }
    pub fn pause(&self) {
        ffi!(sys::elg_video_player_control(self.as_ptr(), 1));
    }
    pub fn toggle(&self) {
        ffi!(sys::elg_video_player_control(self.as_ptr(), 2));
    }
    /// Pauses and goes back to the start.
    pub fn stop(&self) {
        ffi!(sys::elg_video_player_control(self.as_ptr(), 3));
    }
    pub fn seek(&self, seconds: f64) {
        ffi!(sys::elg_video_player_seek(self.as_ptr(), seconds));
    }
    /// 0...1
    pub fn set_volume(&self, volume: f32) {
        ffi!(sys::elg_video_player_set_volume(self.as_ptr(), volume));
    }
    pub fn set_muted(&self, muted: bool) {
        ffi!(sys::elg_video_player_set_muted(self.as_ptr(), muted));
    }
    pub fn set_looping(&self, looping: bool) {
        ffi!(sys::elg_video_player_set_looping(self.as_ptr(), looping));
    }
    pub fn is_playing(&self) -> bool {
        ffi!(sys::elg_video_player_flag(self.as_ptr(), 0))
    }
    /// The duration and size are known.
    pub fn is_ready(&self) -> bool {
        ffi!(sys::elg_video_player_flag(self.as_ptr(), 1))
    }
    pub fn is_muted(&self) -> bool {
        ffi!(sys::elg_video_player_flag(self.as_ptr(), 2))
    }
    pub fn is_looping(&self) -> bool {
        ffi!(sys::elg_video_player_flag(self.as_ptr(), 3))
    }
    pub fn time(&self) -> f64 {
        ffi!(sys::elg_video_player_value(self.as_ptr(), 0))
    }
    pub fn duration(&self) -> f64 {
        ffi!(sys::elg_video_player_value(self.as_ptr(), 1))
    }
    pub fn volume(&self) -> f32 {
        ffi!(sys::elg_video_player_value(self.as_ptr(), 2)) as f32
    }
    /// Pixels; zero until ready.
    pub fn natural_size(&self) -> Size {
        Size::from_c(ffi!(sys::elg_video_player_natural_size(self.as_ptr())))
    }
    pub fn error(&self) -> Option<String> {
        error_string(|sink, ctx| unsafe { sys::elg_video_player_error(self.as_ptr(), sink, ctx) })
    }
    /// The position, 0...1, for a Slider.
    pub fn progress_binding(&self) -> Binding<f32> {
        Binding::from_cpp(ffi!(sys::elg_video_player_binding(self.as_ptr(), 0)))
    }
    pub fn volume_binding(&self) -> Binding<f32> {
        Binding::from_cpp(ffi!(sys::elg_video_player_binding(self.as_ptr(), 1)))
    }
}

/// Shows a VideoPlayer's frames.
pub struct VideoView {
    player: VideoPlayer,
    mode: ContentMode,
}

#[elegance_macros::elg_builder]
impl VideoView {
    pub fn new(player: &VideoPlayer) -> Self {
        Self {
            player: player.clone(),
            mode: ContentMode::Fit,
        }
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
}

impl ElgNew<(&VideoPlayer,)> for VideoView {
    type Out = VideoView;
    fn elg_new((player,): (&VideoPlayer,)) -> Self {
        VideoView::new(player)
    }
}

impl ElgNew<(VideoPlayer,)> for VideoView {
    type Out = VideoView;
    fn elg_new((player,): (VideoPlayer,)) -> Self {
        VideoView {
            player,
            mode: ContentMode::Fit,
        }
    }
}

impl IntoView for VideoView {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::elg_video_view(
            self.player.as_ptr(),
            self.mode as i32
        )))
    }
}

/// Play/pause, a position slider, time, and volume for a VideoPlayer.
pub struct VideoControls {
    _private: (),
}

impl ElgNew<(&VideoPlayer,)> for VideoControls {
    type Out = View;
    fn elg_new((player,): (&VideoPlayer,)) -> View {
        View::from_raw(ffi!(sys::elg_video_controls(player.as_ptr())))
    }
}

impl ElgNew<(VideoPlayer,)> for VideoControls {
    type Out = View;
    fn elg_new((player,): (VideoPlayer,)) -> View {
        Self::elg_new((&player,))
    }
}

/// Pixel formats for `VideoFeed::push`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VideoFormat {
    #[default]
    Bgra,
    Nv12,
    I420,
}

/// Frames you produce (a decoder, a remote camera), shown by `VideoFeedView`. Push from any
/// thread.
pub struct VideoFeed {
    ptr: NonNull<sys::VideoFeed>,
}
handle!(
    VideoFeed,
    VideoFeed,
    elg_video_feed_clone,
    elg_video_feed_free
);
unsafe impl Send for VideoFeed {}
unsafe impl Sync for VideoFeed {}

impl PartialEq for VideoFeed {
    fn eq(&self, other: &Self) -> bool {
        unsafe { sys::elg_video_feed_eq(self.as_ptr(), other.as_ptr()) }
    }
}

#[elegance_macros::elg_builder]
impl VideoFeed {
    pub fn new() -> Result<VideoFeed, Error> {
        Self::from_raw(unsafe { sys::elg_video_feed_new() })
    }
    /// A new frame. `full_range`: YUV in 0...255 rather than 16...235.
    pub fn push(
        &self,
        format: impl Into<VideoFormat>,
        width: u32,
        height: u32,
        pixels: &[u8],
        full_range: bool,
    ) {
        ffi!(sys::elg_video_feed_push(
            self.as_ptr(),
            format.into() as i32,
            width,
            height,
            pixels.as_ptr(),
            pixels.len(),
            full_range
        ));
    }
    /// No picture (black) until the next push.
    pub fn clear(&self) {
        ffi!(sys::elg_video_feed_clear(self.as_ptr()));
    }
    /// Pixels of the frames; zero before the first. Reactive (UI thread).
    pub fn size(&self) -> Size {
        Size::from_c(ffi!(sys::elg_video_feed_size(self.as_ptr())))
    }
    pub fn has_frame(&self) -> bool {
        ffi!(sys::elg_video_feed_has_frame(self.as_ptr()))
    }
}

pub struct VideoFeedView {
    feed: VideoFeed,
    mode: ContentMode,
    mirrored: bool,
}

#[elegance_macros::elg_builder]
impl VideoFeedView {
    pub fn new(feed: &VideoFeed) -> Self {
        Self {
            feed: feed.clone(),
            mode: ContentMode::Fit,
            mirrored: false,
        }
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
    pub fn mirrored(mut self, mirrored: bool) -> Self {
        self.mirrored = mirrored;
        self
    }
}

impl ElgNew<(&VideoFeed,)> for VideoFeedView {
    type Out = VideoFeedView;
    fn elg_new((feed,): (&VideoFeed,)) -> Self {
        VideoFeedView::new(feed)
    }
}

impl IntoView for VideoFeedView {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::elg_video_feed_view(
            self.feed.as_ptr(),
            self.mode as i32,
            self.mirrored
        )))
    }
}

/// Where the producer's first row goes when displaying an [`GpuImage`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GpuImageOrigin {
    #[default]
    TopLeft,
    BottomLeft,
}

/// An externally produced GPU image with variable frame dimensions and a fixed origin.
///
/// Each accepted frame may have different nonzero dimensions. The shared source keeps its
/// identity and updates its reactive natural size asynchronously on the UI thread.
///
/// Clones share one synchronized native handle. Publication is available only through the
/// unsafe native API in `elegance_sys::gpu_image`; there is no safe generic publish.
/// Views retain their source independently. Stop/join producers and remove views before
/// renderer shutdown, retaining callback state until all accepted frames are released.
#[derive(Clone)]
pub struct GpuImage {
    inner: Arc<GpuImageInner>,
}

struct GpuImageInner {
    ptr: NonNull<sys::gpu_image::GpuImage>,
}

// SAFETY: native publish/clear calls on a live handle are synchronized. Arc keeps the
// handle alive across safe calls and destroys it only after the last owner is dropped.
unsafe impl Send for GpuImageInner {}
unsafe impl Sync for GpuImageInner {}

impl Drop for GpuImageInner {
    fn drop(&mut self) {
        unsafe { sys::gpu_image::elg_gpu_image_destroy(self.ptr.as_ptr()) }
    }
}

impl GpuImage {
    /// Creates an image with an initial natural size of `width` × `height` pixels (both nonzero).
    /// Does not publish or import a frame; accepted frames may have different dimensions.
    pub fn new(width: u32, height: u32, origin: GpuImageOrigin) -> Result<Self, Error> {
        use sys::gpu_image as raw;
        let origin = match origin {
            GpuImageOrigin::TopLeft => raw::ORIGIN_TOP_LEFT,
            GpuImageOrigin::BottomLeft => raw::ORIGIN_BOTTOM_LEFT,
        };
        let ptr = NonNull::new(unsafe { raw::elg_gpu_image_create(width, height, origin) })
            .ok_or_else(|| {
                Error::new(
                    crate::ErrorKind::Other,
                    "could not create external GPU image",
                )
            })?;
        Ok(Self {
            inner: Arc::new(GpuImageInner { ptr }),
        })
    }

    /// Returns a borrowed native handle for unsafe low-level interop.
    ///
    /// Do not destroy it. Keep an owning clone alive for every raw call; the pointer does
    /// not extend its lifetime. Raw use is unsafe and must honor the native publication,
    /// resource lifetime, callback, and GPU synchronization contract. Callbacks must not
    /// reenter this API, including `clear`. Destroying the handle is not a callback drain.
    pub fn as_raw(&self) -> *mut sys::gpu_image::GpuImage {
        self.inner.ptr.as_ptr()
    }

    /// Removes the current frame. Release callbacks do not imply GPU completion;
    /// views/snapshots may retain frames, so this is not a drain.
    pub fn clear(&self) {
        unsafe { sys::gpu_image::elg_gpu_image_clear(self.as_raw()) }
    }
}

/// An [`GpuImage`], fitted into (or filling, clipped to) the offered space, on black.
pub struct GpuImageView {
    image: GpuImage,
    mode: ContentMode,
}

#[elegance_macros::elg_builder]
impl GpuImageView {
    pub fn new(image: &GpuImage) -> Self {
        Self {
            image: image.clone(),
            mode: ContentMode::Fit,
        }
    }
    pub fn mode(mut self, mode: impl Into<ContentMode>) -> Self {
        self.mode = mode.into();
        self
    }
}

impl ElgNew<(&GpuImage,)> for GpuImageView {
    type Out = GpuImageView;
    fn elg_new((image,): (&GpuImage,)) -> Self {
        Self::new(image)
    }
}

impl ElgNew<(GpuImage,)> for GpuImageView {
    type Out = GpuImageView;
    fn elg_new((image,): (GpuImage,)) -> Self {
        Self {
            image,
            mode: ContentMode::Fit,
        }
    }
}

impl IntoView for GpuImageView {
    fn into_view(self) -> View {
        View::from_raw(ffi!(sys::gpu_image::elg_gpu_image_view(
            self.image.as_raw(),
            self.mode as i32
        )))
    }
}

#[cfg(test)]
mod gpu_image_tests {
    use super::*;

    #[test]
    fn image_is_send_sync_and_clone() {
        fn assert_traits<T: Send + Sync + Clone>() {}
        assert_traits::<GpuImage>();
    }

    #[test]
    fn create_clone_and_clear() {
        let image = GpuImage::new(16, 16, GpuImageOrigin::BottomLeft).unwrap();
        let clone = image.clone();
        assert_eq!(image.as_raw(), clone.as_raw());
        drop(image);
        clone.clear();
    }

    #[test]
    fn rejects_invalid_dimensions() {
        assert!(GpuImage::new(0, 16, GpuImageOrigin::TopLeft).is_err());
        assert!(GpuImage::new(16, 0, GpuImageOrigin::TopLeft).is_err());
    }

    // Compile both owned and borrowed DSL overloads without requiring a live UI renderer.
    #[allow(dead_code)]
    fn dsl_views(image: GpuImage) -> impl IntoView {
        crate::elg! {
            crate::VStack {
                GpuImageView(&image, mode: .fit)
                GpuImageView(image, mode: .fill)
            }
        }
    }
}
