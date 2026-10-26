//! Audio devices, sounds, music, sample streams, and recording.

use std::path::Path;
use std::ptr::NonNull;

use elegance_sys as sys;

use super::error_string;
use crate::error::{self, ffi, Error};
use crate::ffi as cffi;
use crate::reactive::Binding;

/// An audio device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AudioDevice {
    /// For `AudioStream`/`AudioRecorder` device ids.
    pub id: String,
    /// As the system shows it.
    pub name: String,
    pub is_default: bool,
}

pub(super) fn devices(list: unsafe extern "C" fn(sys::ElgClosure)) -> Vec<AudioDevice> {
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
