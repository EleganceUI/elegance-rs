// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// https://mozilla.org/MPL/2.0/.

// Audio, capture, video, networking, and the desktop (dialogs, notifications, tray icons).

#include "shim.h"
#include <elg/gpu_image.h>

#include <cstddef>

using namespace elg;
using namespace elg_shim;

struct ElgDevice {
    ElgStr id;
    ElgStr name;
    bool is_default;
};

// Calls `each` (args: ElgDevice*) per device.
template <class Device>
static void send_devices(const std::vector<Device>& devices, const ElgClosure& each) {
    for (const Device& device : devices) {
        bool is_default = false;
        if constexpr (requires { device.is_default; }) {
            is_default = device.is_default;
        }
        ElgDevice arg{c_str(device.id), c_str(device.name), is_default};
        call_now(each, &arg, nullptr);
    }
}

// --- Audio ---

ELG_EXPORT void elg_audio_output_devices(ElgClosure each) {
    ELG_TRY
    send_devices(audio_output_devices(), each);
    ELG_CATCH_VOID
}

ELG_EXPORT Sound* elg_sound_load_file(ElgStr file) {
    ELG_TRY
    return new Sound(Sound::load(path(file)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Sound* elg_sound_load_data(ElgBytes data) {
    ELG_TRY
    return new Sound(Sound::load(bytes(data)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Sound* elg_sound_clone(const Sound* sound) {
    return new Sound(*sound);
}
ELG_EXPORT void elg_sound_free(Sound* sound) {
    delete sound;
}
ELG_EXPORT void elg_sound_play(const Sound* sound, float volume, float pitch, float pan) {
    ELG_TRY
    sound->play({volume, pitch, pan});
    ELG_CATCH_VOID
}
ELG_EXPORT double elg_sound_duration(const Sound* sound) {
    return sound->duration();
}

ELG_EXPORT AudioPlayer* elg_audio_player_new(ElgStr file) {
    ELG_TRY
    return new AudioPlayer(path(file));
    ELG_CATCH(nullptr)
}
ELG_EXPORT AudioPlayer* elg_audio_player_clone(const AudioPlayer* player) {
    return new AudioPlayer(*player);
}
ELG_EXPORT void elg_audio_player_free(AudioPlayer* player) {
    delete player;
}
// op: 0 play, 1 pause, 2 toggle, 3 stop.
ELG_EXPORT void elg_audio_player_control(const AudioPlayer* player, int32_t op) {
    ELG_TRY
    switch (op) {
    case 0: player->play(); break;
    case 1: player->pause(); break;
    case 2: player->toggle(); break;
    default: player->stop(); break;
    }
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_audio_player_seek(const AudioPlayer* player, double seconds) {
    ELG_TRY
    player->seek(seconds);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_audio_player_set_volume(const AudioPlayer* player, float volume) {
    ELG_TRY
    player->set_volume(volume);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_audio_player_set_looping(const AudioPlayer* player, bool looping) {
    ELG_TRY
    player->set_looping(looping);
    ELG_CATCH_VOID
}
// which: 0 playing, 1 looping.
ELG_EXPORT bool elg_audio_player_flag(const AudioPlayer* player, int32_t which) {
    ELG_TRY
    return which == 0 ? player->is_playing() : player->is_looping();
    ELG_CATCH(false)
}
// which: 0 time, 1 duration, 2 volume.
ELG_EXPORT double elg_audio_player_value(const AudioPlayer* player, int32_t which) {
    ELG_TRY
    switch (which) {
    case 0: return player->time();
    case 1: return player->duration();
    default: return player->volume();
    }
    ELG_CATCH(0.0)
}
// which: 0 time, 1 volume.
ELG_EXPORT Binding<float>* elg_audio_player_binding(const AudioPlayer* player, int32_t which) {
    ELG_TRY
    return new Binding<float>(which == 0 ? player->time_binding() : player->volume_binding());
    ELG_CATCH(nullptr)
}

struct ElgRenderBuffer {
    float* samples;
    size_t count;
};

// `on_render` (args: ElgRenderBuffer*) runs on the audio thread.
ELG_EXPORT AudioStream* elg_audio_stream_new(ElgStr device_id, uint32_t sample_rate, uint32_t channels, ElgClosure on_render) {
    ELG_TRY
    AudioStreamOptions options;
    options.device_id = str(device_id);
    options.sample_rate = sample_rate;
    options.channels = channels;
    if (SharedFn f = share(on_render)) {
        options.on_render = [f](std::span<float> buffer) {
            ElgRenderBuffer arg{buffer.data(), buffer.size()};
            f->call(&arg, nullptr);
        };
    }
    return new AudioStream(std::move(options));
    ELG_CATCH(nullptr)
}
ELG_EXPORT AudioStream* elg_audio_stream_clone(const AudioStream* stream) {
    return new AudioStream(*stream);
}
ELG_EXPORT void elg_audio_stream_free(AudioStream* stream) {
    delete stream;
}
ELG_EXPORT void elg_audio_stream_write(const AudioStream* stream, const float* samples, size_t count) {
    ELG_TRY
    stream->write(std::span<const float>(samples, count));
    ELG_CATCH_VOID
}
// op: 0 clear, 1 start, 2 stop.
ELG_EXPORT void elg_audio_stream_control(const AudioStream* stream, int32_t op) {
    ELG_TRY
    switch (op) {
    case 0: stream->clear(); break;
    case 1: stream->start(); break;
    default: stream->stop(); break;
    }
    ELG_CATCH_VOID
}
ELG_EXPORT size_t elg_audio_stream_queued_frames(const AudioStream* stream) {
    ELG_TRY
    return stream->queued_frames();
    ELG_CATCH(0)
}
ELG_EXPORT void elg_audio_stream_set_volume(const AudioStream* stream, float volume) {
    ELG_TRY
    stream->set_volume(volume);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_audio_stream_error(const AudioStream* stream, ElgStrSink out, void* ctx) {
    ELG_TRY
    sink(out, ctx, stream->error());
    ELG_CATCH_VOID
}
ELG_EXPORT uint32_t elg_audio_stream_sample_rate(const AudioStream* stream) {
    return stream->sample_rate();
}
ELG_EXPORT uint32_t elg_audio_stream_channels(const AudioStream* stream) {
    return stream->channels();
}

// --- Capture ---

ELG_EXPORT void elg_audio_input_devices(ElgClosure each) {
    ELG_TRY
    send_devices(audio_input_devices(), each);
    ELG_CATCH_VOID
}

struct ElgSamples {
    const float* samples;
    size_t count;
};

// `on_samples` (args: ElgSamples*) runs on the audio thread.
ELG_EXPORT AudioRecorder* elg_audio_recorder_new(ElgStr device_id, uint32_t sample_rate, uint32_t channels,
                                                 ElgClosure on_samples, bool keeps_samples) {
    ELG_TRY
    AudioRecorderOptions options;
    options.device_id = str(device_id);
    options.sample_rate = sample_rate;
    options.channels = channels;
    if (SharedFn f = share(on_samples)) {
        options.on_samples = [f](std::span<const float> samples) {
            ElgSamples arg{samples.data(), samples.size()};
            f->call(&arg, nullptr);
        };
    }
    options.keeps_samples = keeps_samples;
    return new AudioRecorder(std::move(options));
    ELG_CATCH(nullptr)
}
ELG_EXPORT AudioRecorder* elg_audio_recorder_clone(const AudioRecorder* recorder) {
    return new AudioRecorder(*recorder);
}
ELG_EXPORT void elg_audio_recorder_free(AudioRecorder* recorder) {
    delete recorder;
}
// op: 0 start, 1 stop, 2 toggle, 3 clear.
ELG_EXPORT void elg_audio_recorder_control(const AudioRecorder* recorder, int32_t op) {
    ELG_TRY
    switch (op) {
    case 0: recorder->start(); break;
    case 1: recorder->stop(); break;
    case 2: recorder->toggle(); break;
    default: recorder->clear(); break;
    }
    ELG_CATCH_VOID
}
ELG_EXPORT bool elg_audio_recorder_is_recording(const AudioRecorder* recorder) {
    ELG_TRY
    return recorder->is_recording();
    ELG_CATCH(false)
}
ELG_EXPORT double elg_audio_recorder_duration(const AudioRecorder* recorder) {
    ELG_TRY
    return recorder->duration();
    ELG_CATCH(0.0)
}
ELG_EXPORT float elg_audio_recorder_level(const AudioRecorder* recorder) {
    ELG_TRY
    return recorder->level();
    ELG_CATCH(0.f)
}
ELG_EXPORT void elg_audio_recorder_error(const AudioRecorder* recorder, ElgStrSink out, void* ctx) {
    ELG_TRY
    sink(out, ctx, recorder->error());
    ELG_CATCH_VOID
}
ELG_EXPORT uint32_t elg_audio_recorder_sample_rate(const AudioRecorder* recorder) {
    return recorder->sample_rate();
}
ELG_EXPORT uint32_t elg_audio_recorder_channels(const AudioRecorder* recorder) {
    return recorder->channels();
}
// Calls `each` (args: ElgSamples*) once with all the samples.
ELG_EXPORT void elg_audio_recorder_samples(const AudioRecorder* recorder, ElgClosure each) {
    ELG_TRY
    const std::vector<float> samples = recorder->samples();
    ElgSamples arg{samples.data(), samples.size()};
    call_now(each, &arg, nullptr);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_audio_recorder_save_wav(const AudioRecorder* recorder, ElgStr file) {
    ELG_TRY
    recorder->save_wav(path(file));
    ELG_CATCH_VOID
}
// Calls `each` (args: ElgBytes*) once with the WAV file's bytes.
ELG_EXPORT void elg_audio_recorder_wav(const AudioRecorder* recorder, ElgClosure each) {
    ELG_TRY
    const std::vector<unsigned char> wav = recorder->wav();
    ElgBytes arg{wav.data(), wav.size()};
    call_now(each, &arg, nullptr);
    ELG_CATCH_VOID
}
ELG_EXPORT Sound* elg_audio_recorder_sound(const AudioRecorder* recorder) {
    ELG_TRY
    return new Sound(recorder->sound());
    ELG_CATCH(nullptr)
}

ELG_EXPORT void elg_camera_devices(ElgClosure each) {
    ELG_TRY
    send_devices(camera_devices(), each);
    ELG_CATCH_VOID
}

ELG_EXPORT Camera* elg_camera_new(ElgStr device_id, uint32_t width, uint32_t height, bool mirrored) {
    ELG_TRY
    return new Camera({str(device_id), width, height, mirrored});
    ELG_CATCH(nullptr)
}
ELG_EXPORT Camera* elg_camera_clone(const Camera* camera) {
    return new Camera(*camera);
}
ELG_EXPORT void elg_camera_free(Camera* camera) {
    delete camera;
}
ELG_EXPORT void elg_camera_start(const Camera* camera, bool start) {
    ELG_TRY
    if (start) {
        camera->start();
    } else {
        camera->stop();
    }
    ELG_CATCH_VOID
}
ELG_EXPORT bool elg_camera_is_running(const Camera* camera) {
    ELG_TRY
    return camera->is_running();
    ELG_CATCH(false)
}
ELG_EXPORT ElgSize elg_camera_size(const Camera* camera) {
    ELG_TRY
    const Size s = camera->size();
    return {s.width, s.height};
    ELG_CATCH(ElgSize{})
}
ELG_EXPORT void elg_camera_error(const Camera* camera, ElgStrSink out, void* ctx) {
    ELG_TRY
    sink(out, ctx, camera->error());
    ELG_CATCH_VOID
}
ELG_EXPORT bool elg_camera_mirrored(const Camera* camera) {
    return camera->mirrored();
}
// Calls `each` (args: ElgBytes*) with a PNG of the latest frame; returns false if there's none.
ELG_EXPORT bool elg_camera_snapshot(const Camera* camera, ElgClosure each) {
    ELG_TRY
    const auto png = camera->snapshot();
    if (!png) {
        return false;
    }
    ElgBytes arg{png->data(), png->size()};
    call_now(each, &arg, nullptr);
    return true;
    ELG_CATCH(false)
}
ELG_EXPORT View* elg_camera_view(const Camera* camera, int32_t mode) {
    ELG_TRY
    return out(CameraView(*camera, static_cast<ContentMode>(mode)));
    ELG_CATCH(nullptr)
}

// --- Video ---

ELG_EXPORT VideoPlayer* elg_video_player_new(ElgStr source) {
    ELG_TRY
    return new VideoPlayer(str(source));
    ELG_CATCH(nullptr)
}
ELG_EXPORT VideoPlayer* elg_video_player_new_file(ElgStr file) {
    ELG_TRY
    return new VideoPlayer(path(file));
    ELG_CATCH(nullptr)
}
ELG_EXPORT VideoPlayer* elg_video_player_clone(const VideoPlayer* player) {
    return new VideoPlayer(*player);
}
ELG_EXPORT void elg_video_player_free(VideoPlayer* player) {
    delete player;
}
// op: 0 play, 1 pause, 2 toggle, 3 stop.
ELG_EXPORT void elg_video_player_control(const VideoPlayer* player, int32_t op) {
    ELG_TRY
    switch (op) {
    case 0: player->play(); break;
    case 1: player->pause(); break;
    case 2: player->toggle(); break;
    default: player->stop(); break;
    }
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_video_player_seek(const VideoPlayer* player, double seconds) {
    ELG_TRY
    player->seek(seconds);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_video_player_set_volume(const VideoPlayer* player, float volume) {
    ELG_TRY
    player->set_volume(volume);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_video_player_set_muted(const VideoPlayer* player, bool muted) {
    ELG_TRY
    player->set_muted(muted);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_video_player_set_looping(const VideoPlayer* player, bool looping) {
    ELG_TRY
    player->set_looping(looping);
    ELG_CATCH_VOID
}
// which: 0 playing, 1 ready, 2 muted, 3 looping.
ELG_EXPORT bool elg_video_player_flag(const VideoPlayer* player, int32_t which) {
    ELG_TRY
    switch (which) {
    case 0: return player->is_playing();
    case 1: return player->is_ready();
    case 2: return player->is_muted();
    default: return player->is_looping();
    }
    ELG_CATCH(false)
}
// which: 0 time, 1 duration, 2 volume.
ELG_EXPORT double elg_video_player_value(const VideoPlayer* player, int32_t which) {
    ELG_TRY
    switch (which) {
    case 0: return player->time();
    case 1: return player->duration();
    default: return player->volume();
    }
    ELG_CATCH(0.0)
}
ELG_EXPORT ElgSize elg_video_player_natural_size(const VideoPlayer* player) {
    ELG_TRY
    const Size s = player->natural_size();
    return {s.width, s.height};
    ELG_CATCH(ElgSize{})
}
ELG_EXPORT void elg_video_player_error(const VideoPlayer* player, ElgStrSink out, void* ctx) {
    ELG_TRY
    sink(out, ctx, player->error());
    ELG_CATCH_VOID
}
// which: 0 progress, 1 volume.
ELG_EXPORT Binding<float>* elg_video_player_binding(const VideoPlayer* player, int32_t which) {
    ELG_TRY
    return new Binding<float>(which == 0 ? player->progress_binding() : player->volume_binding());
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_video_view(const VideoPlayer* player, int32_t mode) {
    ELG_TRY
    return out(VideoView(*player, static_cast<ContentMode>(mode)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_video_controls(const VideoPlayer* player) {
    ELG_TRY
    return out(VideoControls(*player));
    ELG_CATCH(nullptr)
}

// --- External GPU images ---

ELG_EXPORT elg::View* elg_gpu_image_view(const ElgGpuImage* image, int32_t mode) {
    ELG_TRY
    return new elg::View(elg::GpuImageView(image, static_cast<elg::ContentMode>(mode)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT VideoFeed* elg_video_feed_new() {
    ELG_TRY
    return new VideoFeed();
    ELG_CATCH(nullptr)
}
ELG_EXPORT VideoFeed* elg_video_feed_clone(const VideoFeed* feed) {
    return new VideoFeed(*feed);
}
ELG_EXPORT void elg_video_feed_free(VideoFeed* feed) {
    delete feed;
}
ELG_EXPORT bool elg_video_feed_eq(const VideoFeed* a, const VideoFeed* b) {
    return *a == *b;
}
ELG_EXPORT void elg_video_feed_push(const VideoFeed* feed, int32_t format, uint32_t width, uint32_t height,
                                    const uint8_t* pixels, size_t count, bool full_range) {
    ELG_TRY
    feed->push(static_cast<VideoFeed::Format>(format), width, height, std::vector<std::uint8_t>(pixels, pixels + count),
               full_range);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_video_feed_clear(const VideoFeed* feed) {
    ELG_TRY
    feed->clear();
    ELG_CATCH_VOID
}
ELG_EXPORT ElgSize elg_video_feed_size(const VideoFeed* feed) {
    ELG_TRY
    const Size s = feed->size();
    return {s.width, s.height};
    ELG_CATCH(ElgSize{})
}
ELG_EXPORT bool elg_video_feed_has_frame(const VideoFeed* feed) {
    ELG_TRY
    return feed->has_frame();
    ELG_CATCH(false)
}
ELG_EXPORT View* elg_video_feed_view(const VideoFeed* feed, int32_t mode, bool mirrored) {
    ELG_TRY
    return out(VideoFeedView(*feed, static_cast<ContentMode>(mode), mirrored));
    ELG_CATCH(nullptr)
}

// --- Desktop ---

struct ElgFileFilter {
    ElgStr name;
    const ElgStr* extensions;
    size_t extension_count;
};

struct ElgPaths {
    const ElgStr* paths;
    size_t count;
};

static std::vector<FileFilter> filters(const ElgFileFilter* items, size_t count) {
    std::vector<FileFilter> result;
    for (size_t i = 0; i < count; ++i) {
        FileFilter filter{str(items[i].name), {}};
        for (size_t j = 0; j < items[i].extension_count; ++j) {
            filter.extensions.push_back(str(items[i].extensions[j]));
        }
        result.push_back(std::move(filter));
    }
    return result;
}

static void send_paths(const SharedFn& f, const std::vector<std::filesystem::path>& paths) {
    std::vector<std::string> names;
    for (const auto& p : paths) {
        names.push_back(utf8(p));
    }
    std::vector<ElgStr> items;
    for (const auto& n : names) {
        items.push_back(c_str(n));
    }
    ElgPaths arg{items.data(), items.size()};
    f->call(&arg, nullptr);
}

// `done` args: ElgPaths* (empty if cancelled).
ELG_EXPORT void elg_choose_files(const ElgFileFilter* fs, size_t filter_count, ElgStr directory, bool multiple, ElgClosure done) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(done);
    choose_files({filters(fs, filter_count), path(directory), multiple},
                 [f](std::vector<std::filesystem::path> paths) { send_paths(f, paths); });
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_choose_save_file(const ElgFileFilter* fs, size_t filter_count, ElgStr directory, ElgStr name,
                                     ElgClosure done) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(done);
    choose_save_file({filters(fs, filter_count), path(directory), str(name)}, [f](std::optional<std::filesystem::path> p) {
        std::vector<std::filesystem::path> paths;
        if (p) {
            paths.push_back(*p);
        }
        send_paths(f, paths);
    });
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_choose_folders(ElgStr directory, bool multiple, ElgClosure done) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(done);
    choose_folders({path(directory), multiple}, [f](std::vector<std::filesystem::path> paths) { send_paths(f, paths); });
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_notify(ElgStr title, ElgStr body, ElgClosure on_click) {
    ELG_TRY
    notify({str(title), str(body), action(on_click)});
    ELG_CATCH_VOID
}

struct ElgTrayMenuItem {
    ElgStr title;
    ElgClosure action;
    int32_t checked; // -1: not a check box, 0/1: initially unchecked/checked
    ElgClosure toggled; // args: bool*
    bool enabled;
    const ElgTrayMenuItem* submenu;
    size_t submenu_count;
};

static std::vector<TrayMenuItem> tray_menu(const ElgTrayMenuItem* items, size_t count) {
    std::vector<TrayMenuItem> result;
    result.reserve(count);
    for (size_t i = 0; i < count; ++i) {
        TrayMenuItem item;
        item.title = str(items[i].title);
        item.action = action(items[i].action);
        if (items[i].checked >= 0) {
            item.checked = items[i].checked != 0;
        }
        item.toggled = consumer<bool>(items[i].toggled);
        item.enabled = items[i].enabled;
        item.submenu = tray_menu(items[i].submenu, items[i].submenu_count);
        result.push_back(std::move(item));
    }
    return result;
}

ELG_EXPORT TrayIcon* elg_tray_icon_new(const ElgImageSource* icon, ElgStr tooltip, const ElgTrayMenuItem* menu, size_t count) {
    ELG_TRY
    return new TrayIcon({image_source(*icon), str(tooltip), tray_menu(menu, count)});
    ELG_CATCH(nullptr)
}
ELG_EXPORT TrayIcon* elg_tray_icon_clone(const TrayIcon* icon) {
    return new TrayIcon(*icon);
}
ELG_EXPORT void elg_tray_icon_free(TrayIcon* icon) {
    delete icon;
}
ELG_EXPORT void elg_tray_icon_set_tooltip(const TrayIcon* icon, ElgStr tooltip) {
    ELG_TRY
    icon->set_tooltip(str(tooltip));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_tray_icon_set_menu(const TrayIcon* icon, const ElgTrayMenuItem* menu, size_t count) {
    ELG_TRY
    icon->set_menu(tray_menu(menu, count));
    ELG_CATCH_VOID
}

// --- ABI layout (this file's structs and elg/gpu_image.h's) ---

ELG_EXPORT void elg_layouts_platform(ElgStrSink out, void* ctx) {
    ELG_LAYOUT(ElgDevice);
    ELG_LAYOUT(ElgRenderBuffer);
    ELG_LAYOUT(ElgSamples);
    ELG_LAYOUT(ElgFileFilter);
    ELG_LAYOUT(ElgPaths);
    ELG_LAYOUT(ElgTrayMenuItem);
    ELG_LAYOUT(ElgGpuImageFence);
    ELG_LAYOUT(ElgGpuImageRelease);
    ELG_LAYOUT(ElgGpuImageFrame);
}
