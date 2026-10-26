//! Video files, frames you produce, and their views.

use std::path::Path;
use std::ptr::NonNull;

use elegance_sys as sys;

use super::error_string;
use crate::dsl::ElgNew;
use crate::error::{ffi, Error};
use crate::ffi as cffi;
use crate::geometry::{ContentMode, Size};
use crate::reactive::Binding;
use crate::view::{IntoView, View};

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
