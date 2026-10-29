// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// https://mozilla.org/MPL/2.0/.

// Errors, view handles, reactive state, the environment, App, animations.

#include "shim.h"

#include <exception>

using namespace elg;
using namespace elg_shim;

static_assert(sizeof(Color) == sizeof(ElgColor));
static_assert(sizeof(Theme) == 34 * sizeof(ElgColor) + 7 * sizeof(float));
static_assert(sizeof(Size) == sizeof(ElgSize));
static_assert(sizeof(LayoutChild) == sizeof(void*), "Rust views a span of LayoutChild as a slice of pointers");

// --- Errors ---

namespace {

enum ErrorKind : int32_t {
    no_error = 0,
    panic_error = 1,
    usage_error = 2,
    resource_error = 3,
    gpu_error = 4,
    platform_error = 5,
    elegance_error = 6,
    other_error = 7,
};

thread_local int32_t g_error_kind = no_error;
thread_local std::string g_error_message;

} // namespace

void elg_shim::capture_error() noexcept {
    try {
        throw;
    } catch (const RustPanic&) {
        g_error_kind = panic_error;
        g_error_message = "a Rust callback panicked";
    } catch (const UsageError& e) {
        g_error_kind = usage_error;
        g_error_message = e.what();
    } catch (const ResourceError& e) {
        g_error_kind = resource_error;
        g_error_message = e.what();
    } catch (const GpuError& e) {
        g_error_kind = gpu_error;
        g_error_message = e.what();
    } catch (const PlatformError& e) {
        g_error_kind = platform_error;
        g_error_message = e.what();
    } catch (const Error& e) {
        g_error_kind = elegance_error;
        g_error_message = e.what();
    } catch (const std::exception& e) {
        g_error_kind = other_error;
        g_error_message = e.what();
    } catch (...) {
        g_error_kind = other_error;
        g_error_message = "unknown C++ exception";
    }
}

// The pending error's kind (0: none), with its message sent to `sink`; clears it.
ELG_EXPORT int32_t elg_take_error(ElgStrSink out, void* ctx) {
    const int32_t kind = std::exchange(g_error_kind, no_error);
    if (kind != no_error) {
        sink(out, ctx, g_error_message);
        g_error_message.clear();
    }
    return kind;
}

ELG_EXPORT bool elg_error_pending() {
    return g_error_kind != no_error;
}

// Lets Rust raise a usage error from inside a callback (e.g. a missing environment).
ELG_EXPORT void elg_string_assign(std::string* target, ElgStr value) {
    target->assign(view(value));
}

ELG_EXPORT ElgStr elg_string_view(const std::string* s) {
    return c_str(*s);
}

// --- Views ---

ELG_EXPORT View* elg_view_empty() {
    return new View();
}

ELG_EXPORT View* elg_view_clone(const View* view) {
    return new View(*view);
}

ELG_EXPORT void elg_view_free(View* view) {
    delete view;
}

// Moves `take_view` into `*target` (a closure's View result).
ELG_EXPORT void elg_view_move_into(View* target, View* take_view) {
    *target = std::move(*take_view);
    delete take_view;
}

// A Rust component: `body` (ret: View*) runs each time the view is mounted.
ELG_EXPORT bool elg_view_is_empty(const View* view) {
    return view->is_empty();
}

ELG_EXPORT View* elg_view_component(ElgClosure body) {
    ELG_TRY
    return out(View(RustComponent{share(body)}));
    ELG_CATCH(nullptr)
}

// --- Reactive state ---
//
// Rust keeps the values; each State/Computed/Binding has a C++ node for dependency tracking and
// lifetime. Nodes belong to the current Scope (the mounted view tree, If branch, ForEach item...),
// so they end with the views that made them. `owner`'s drop frees the Rust side's slot.

namespace {

using OwnerNode = detail::SignalNode<SharedFn>;

std::vector<std::shared_ptr<detail::ReactiveNode>>& roots() {
    // Leaked: nodes made outside any scope (before App::run) live for the whole program, and
    // must not be destroyed after the Rust side's thread-locals are gone.
    static auto* nodes = new std::vector<std::shared_ptr<detail::ReactiveNode>>();
    return *nodes;
}

void adopt(std::shared_ptr<detail::ReactiveNode> node) {
    if (detail::Scope* scope = detail::current_scope()) {
        scope->adopt(std::move(node));
    } else {
        roots().push_back(std::move(node));
    }
}

} // namespace

// A signal node owning `owner`. Returns the node, for elg_node_track / elg_node_notify.
ELG_EXPORT void* elg_signal_new(ElgClosure owner) {
    ELG_TRY
    auto node = std::make_shared<OwnerNode>(std::make_shared<const RustFn>(owner));
    void* raw = static_cast<detail::ReactiveNode*>(node.get());
    adopt(std::move(node));
    return raw;
    ELG_CATCH(nullptr)
}

// A computed node. `compute` (ret: uint64_t*) brings the Rust value up to date and returns a
// version that changes whenever the value does.
ELG_EXPORT void* elg_computed_new(ElgClosure compute) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(compute);
    auto node = std::make_shared<detail::ComputedNode<std::uint64_t>>([f] {
        std::uint64_t version = 0;
        f->call(nullptr, &version);
        return version;
    });
    void* raw = static_cast<detail::ReactiveNode*>(node.get());
    adopt(std::move(node));
    return raw;
    ELG_CATCH(nullptr)
}

// A read: subscribes the running computed/effect, bringing a computed up to date first.
ELG_EXPORT void elg_node_track(void* node) {
    ELG_TRY
    static_cast<detail::ReactiveNode*>(node)->track();
    ELG_CATCH_VOID
}

// A write happened (UI thread only).
ELG_EXPORT void elg_node_notify(void* node) {
    ELG_TRY
    detail::check_ui_thread();
    static_cast<detail::ReactiveNode*>(node)->notify_changed();
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_check_ui_thread() {
    ELG_TRY
    detail::check_ui_thread();
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_effect(ElgClosure function) {
    ELG_TRY
    effect(action(function));
    ELG_CATCH_VOID
}

// Runs `function` (borrowed) without tracking reads.
ELG_EXPORT void elg_untrack(ElgClosure function) {
    ELG_TRY
    untrack([&] { call_now(function, nullptr, nullptr); });
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_batch(ElgClosure function) {
    ELG_TRY
    batch([&] { call_now(function, nullptr, nullptr); });
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_with_animation(const Animation* animation, ElgClosure changes) {
    ELG_TRY
    with_animation(*animation, [&] { call_now(changes, nullptr, nullptr); });
    ELG_CATCH_VOID
}

// --- Bindings to the C++ value types controls take ---

template <class T>
static Binding<T>* make_binding(ElgClosure get, ElgClosure set) {
    SharedFn g = std::make_shared<const RustFn>(get);
    SharedFn s = std::make_shared<const RustFn>(set);
    return new Binding<T>(
        [g] {
            T value{};
            g->call(nullptr, &value);
            return value;
        },
        [s](T value) { s->call(&value, nullptr); });
}

// get: ret bool*/float*/int32_t*/std::string*. set: args the same (std::string*: read it with
// elg_string_view).
ELG_EXPORT Binding<bool>* elg_binding_bool(ElgClosure get, ElgClosure set) {
    return make_binding<bool>(get, set);
}
ELG_EXPORT Binding<float>* elg_binding_f32(ElgClosure get, ElgClosure set) {
    return make_binding<float>(get, set);
}
ELG_EXPORT Binding<int>* elg_binding_i32(ElgClosure get, ElgClosure set) {
    return make_binding<int>(get, set);
}
ELG_EXPORT Binding<std::string>* elg_binding_string(ElgClosure get, ElgClosure set) {
    return make_binding<std::string>(get, set);
}
ELG_EXPORT Binding<double>* elg_binding_f64(ElgClosure get, ElgClosure set) {
    return make_binding<double>(get, set);
}

// Writes an optional float into a closure's std::optional<float> result.
ELG_EXPORT void elg_opt_float_assign(std::optional<float>* target, ElgOptLen value) {
    *target = value.has ? std::optional<float>(value.value) : std::nullopt;
}
static_assert(sizeof(EdgeInsets) == sizeof(ElgEdgeInsets), "Rust writes EdgeInsets results as ElgEdgeInsets");

// Bindings made by Elegance (AudioPlayer::time_binding, ...), for the Rust side to read and write.
ELG_EXPORT float elg_binding_f32_get(const Binding<float>* binding) {
    ELG_TRY
    return binding->get();
    ELG_CATCH(0.f)
}
ELG_EXPORT void elg_binding_f32_set(const Binding<float>* binding, float value) {
    ELG_TRY
    binding->set(value);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_binding_f32_free(Binding<float>* binding) {
    delete binding;
}

// --- Environment ---

namespace {

// Environment values set from Rust live in one entry: a persistent map owned by the Rust side.
struct RustEnvironmentKey {
    using Value = SharedFn;
    static constexpr std::string_view name = "rust.environment";
    static SharedFn default_value(const EnvironmentValues&) { return nullptr; }
};

} // namespace

ELG_EXPORT EnvironmentValues* elg_env_current() {
    ELG_TRY
    const EnvironmentValues* environment = detail::current_environment();
    if (!environment) {
        detail::throw_no_environment();
    }
    return new EnvironmentValues(*environment);
    ELG_CATCH(nullptr)
}

ELG_EXPORT EnvironmentValues* elg_env_clone(const EnvironmentValues* environment) {
    return new EnvironmentValues(*environment);
}

ELG_EXPORT void elg_env_free(EnvironmentValues* environment) {
    delete environment;
}

ELG_EXPORT ElgColor elg_env_color(const EnvironmentValues* environment, int32_t token) {
    ELG_TRY
    return c_color(environment->color(static_cast<ThemeColor>(token)));
    ELG_CATCH(ElgColor{})
}

ELG_EXPORT void elg_env_font(const EnvironmentValues* environment, ElgStrSink family, void* ctx, float* size,
                             int32_t* weight, bool* italic) {
    ELG_TRY
    const Font font = environment->get<FontKey>();
    sink(family, ctx, font.family());
    *size = font.size();
    *weight = static_cast<int32_t>(font.weight());
    *italic = font.is_italic();
    ELG_CATCH_VOID
}

ELG_EXPORT int32_t elg_env_color_scheme(const EnvironmentValues* environment) {
    ELG_TRY
    return static_cast<int32_t>(environment->get<ColorSchemeKey>());
    ELG_CATCH(0)
}

ELG_EXPORT void elg_env_theme(const EnvironmentValues* environment, Theme* result) {
    ELG_TRY
    *result = environment->get<ThemeKey>();
    ELG_CATCH_VOID
}

ELG_EXPORT ElgColor elg_env_foreground(const EnvironmentValues* environment) {
    ELG_TRY
    return c_color(environment->get<ForegroundKey>());
    ELG_CATCH(ElgColor{})
}

ELG_EXPORT ElgColor elg_env_tint(const EnvironmentValues* environment) {
    ELG_TRY
    return c_color(environment->get<TintKey>());
    ELG_CATCH(ElgColor{})
}

ELG_EXPORT bool elg_env_disabled(const EnvironmentValues* environment) {
    ELG_TRY
    return environment->get<DisabledKey>();
    ELG_CATCH(false)
}

ELG_EXPORT ElgHoverDelay elg_env_hover_delay(const EnvironmentValues* environment) {
    ELG_TRY
    const HoverDelay delay = environment->get<HoverDelayKey>();
    return {delay.enter, delay.exit};
    ELG_CATCH(ElgHoverDelay{})
}

ELG_EXPORT WindowHandle* elg_env_window(const EnvironmentValues* environment) {
    ELG_TRY
    return new WindowHandle(environment->get<WindowKey>());
    ELG_CATCH(nullptr)
}

ELG_EXPORT std::shared_ptr<const ButtonStyle>* elg_env_button_style(const EnvironmentValues* environment) {
    ELG_TRY
    return new std::shared_ptr<const ButtonStyle>(environment->get<ButtonStyleKey>());
    ELG_CATCH(nullptr)
}

ELG_EXPORT std::shared_ptr<const ToggleStyle>* elg_env_toggle_style(const EnvironmentValues* environment) {
    ELG_TRY
    return new std::shared_ptr<const ToggleStyle>(environment->get<ToggleStyleKey>());
    ELG_CATCH(nullptr)
}

ELG_EXPORT std::shared_ptr<const TextFieldStyle>* elg_env_text_field_style(const EnvironmentValues* environment) {
    ELG_TRY
    return new std::shared_ptr<const TextFieldStyle>(environment->get<TextFieldStyleKey>());
    ELG_CATCH(nullptr)
}

// Inside a selectable row: whether it's selected.
ELG_EXPORT bool elg_env_row_selected(const EnvironmentValues* environment) {
    ELG_TRY
    return environment->get<RowSelectedKey>();
    ELG_CATCH(false)
}

// The Rust side's map, or null.
ELG_EXPORT void* elg_env_rust_values(const EnvironmentValues* environment) {
    ELG_TRY
    const SharedFn values = environment->get<RustEnvironmentKey>();
    return values ? values->data() : nullptr;
    ELG_CATCH(nullptr)
}

// Setters, for EnvironmentModifier::apply.
ELG_EXPORT void elg_env_set_font(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<FontKey>(prop<Font>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_color_scheme(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<ColorSchemeKey>(prop<ColorScheme>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_theme(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<ThemeKey>(prop<Theme>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_foreground(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<ForegroundKey>(prop<Color>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_tint(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<TintKey>(prop<Color>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_disabled(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<DisabledKey>(prop<bool>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_hover_delay(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<HoverDelayKey>(prop<HoverDelay>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_row_selected(EnvironmentValues* environment, ElgProp value) {
    ELG_TRY
    environment->set<RowSelectedKey>(prop<bool>(value));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_window(EnvironmentValues* environment, const WindowHandle* window) {
    ELG_TRY
    environment->set<WindowKey>(*window);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_button_style(EnvironmentValues* environment, const std::shared_ptr<const ButtonStyle>* style) {
    ELG_TRY
    environment->set<ButtonStyleKey>(*style);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_toggle_style(EnvironmentValues* environment, const std::shared_ptr<const ToggleStyle>* style) {
    ELG_TRY
    environment->set<ToggleStyleKey>(*style);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_text_field_style(EnvironmentValues* environment,
                                             const std::shared_ptr<const TextFieldStyle>* style) {
    ELG_TRY
    environment->set<TextFieldStyleKey>(*style);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_env_set_rust_values(EnvironmentValues* environment, ElgClosure values) {
    ELG_TRY
    environment->set<RustEnvironmentKey>(SharedFn(std::make_shared<const RustFn>(values)));
    ELG_CATCH_VOID
}

// --- Windows ---

ELG_EXPORT WindowHandle* elg_window_clone(const WindowHandle* window) {
    return new WindowHandle(*window);
}
ELG_EXPORT void elg_window_free(WindowHandle* window) {
    delete window;
}
ELG_EXPORT bool elg_window_eq(const WindowHandle* a, const WindowHandle* b) {
    return *a == *b;
}
ELG_EXPORT void elg_window_close(const WindowHandle* window) {
    ELG_TRY
    window->close();
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_window_minimize(const WindowHandle* window) {
    ELG_TRY
    window->minimize();
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_window_toggle_maximized(const WindowHandle* window) {
    ELG_TRY
    window->toggle_maximized();
    ELG_CATCH_VOID
}
ELG_EXPORT bool elg_window_is_maximized(const WindowHandle* window) {
    ELG_TRY
    return window->is_maximized();
    ELG_CATCH(false)
}
ELG_EXPORT bool elg_window_is_active(const WindowHandle* window) {
    ELG_TRY
    return window->is_active();
    ELG_CATCH(false)
}
ELG_EXPORT int32_t elg_window_title_bar(const WindowHandle* window) {
    ELG_TRY
    return static_cast<int32_t>(window->title_bar());
    ELG_CATCH(0)
}
ELG_EXPORT ElgRect elg_window_system_controls_frame(const WindowHandle* window) {
    ELG_TRY
    return c_rect(window->system_controls_frame());
    ELG_CATCH(ElgRect{})
}

ELG_EXPORT View* elg_window_controls(int32_t style, bool minimize, bool maximize) {
    ELG_TRY
    return out(WindowControls({static_cast<WindowControlsStyle>(style), minimize, maximize}));
    ELG_CATCH(nullptr)
}

// --- App ---

struct ElgWindowDesc {
    ElgStr title;
    int32_t width;
    int32_t height;
    bool resizable;
    bool has_min_size;
    ElgSize min_size;
    bool has_background;
    ElgColor background;
    int32_t title_bar;
    bool has_system_controls_position;
    ElgPoint system_controls_position;
    bool transparent;
    float opacity;
    bool fullscreen;
};

ELG_EXPORT App* elg_app_new() {
    ELG_TRY
    return new App();
    ELG_CATCH(nullptr)
}

ELG_EXPORT void elg_app_free(App* app) {
    ELG_TRY
    delete app;
    ELG_CATCH_VOID
}

// `content` (ret: View*) builds the window's root view when it opens.
ELG_EXPORT void elg_app_window(App* app, const ElgWindowDesc* desc, ElgClosure content) {
    ELG_TRY
    WindowDesc window;
    window.title = str(desc->title);
    window.width = desc->width;
    window.height = desc->height;
    window.resizable = desc->resizable;
    if (desc->has_min_size) {
        window.min_size = size(desc->min_size);
    }
    if (desc->has_background) {
        window.background = color(desc->background);
    }
    window.title_bar = static_cast<TitleBar>(desc->title_bar);
    if (desc->has_system_controls_position) {
        window.system_controls_position = point(desc->system_controls_position);
    }
    window.transparent = desc->transparent;
    window.opacity = desc->opacity;
    window.fullscreen = desc->fullscreen;
    SharedFn f = std::make_shared<const RustFn>(content);
    app->window(std::move(window), [f]() -> View {
        View result;
        f->call(nullptr, &result);
        return result;
    });
    ELG_CATCH_VOID
}

ELG_EXPORT int32_t elg_app_run(App* app) {
    ELG_TRY
    return app->run();
    ELG_CATCH(-1)
}

ELG_EXPORT void elg_app_quit(App* app) {
    ELG_TRY
    app->quit();
    ELG_CATCH_VOID
}

// `task` is a Send closure; it runs on the UI thread.
ELG_EXPORT void elg_post_to_ui_thread(ElgClosure task) {
    ELG_TRY
    post_to_ui_thread(action(task));
    ELG_CATCH_VOID
}

ELG_EXPORT const char* elg_version() {
    return version();
}

// --- Animation ---

ELG_EXPORT Animation* elg_animation_linear(double duration) {
    return new Animation(Animation::linear(duration));
}
ELG_EXPORT Animation* elg_animation_ease_in(double duration) {
    return new Animation(Animation::ease_in(duration));
}
ELG_EXPORT Animation* elg_animation_ease_out(double duration) {
    return new Animation(Animation::ease_out(duration));
}
ELG_EXPORT Animation* elg_animation_ease_in_out(double duration) {
    return new Animation(Animation::ease_in_out(duration));
}
ELG_EXPORT Animation* elg_animation_spring(double response, double damping_fraction) {
    return new Animation(Animation::spring(response, damping_fraction));
}
ELG_EXPORT Animation* elg_animation_smooth(double duration) {
    return new Animation(Animation::smooth(duration));
}
// `curve` (args: double* t, ret: double*) maps progress 0...1 to eased progress.
ELG_EXPORT Animation* elg_animation_custom(ElgClosure curve, double duration) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(curve);
    return new Animation(Animation::custom(
        [f](double t) {
            double result = t;
            f->call(&t, &result);
            return result;
        },
        duration));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Animation* elg_animation_delay(const Animation* animation, double seconds) {
    return new Animation(animation->delay(seconds));
}
ELG_EXPORT double elg_animation_progress(const Animation* animation, double seconds) {
    ELG_TRY
    return animation->progress(seconds);
    ELG_CATCH(1.0)
}
ELG_EXPORT bool elg_animation_finished(const Animation* animation, double seconds) {
    ELG_TRY
    return animation->finished(seconds);
    ELG_CATCH(true)
}
ELG_EXPORT Animation* elg_animation_clone(const Animation* animation) {
    return new Animation(*animation);
}
ELG_EXPORT void elg_animation_free(Animation* animation) {
    delete animation;
}

// --- Transition ---

struct ElgPose {
    float opacity, scale, x, y;
};

ELG_EXPORT Transition* elg_transition_opacity() {
    return new Transition(Transition::opacity());
}
ELG_EXPORT Transition* elg_transition_identity() {
    return new Transition(Transition::identity());
}
ELG_EXPORT Transition* elg_transition_scale(float from, ElgAlignment anchor) {
    return new Transition(Transition::scale(from, alignment(anchor)));
}
ELG_EXPORT Transition* elg_transition_offset(float x, float y) {
    return new Transition(Transition::offset(x, y));
}
ELG_EXPORT Transition* elg_transition_move(uint32_t edge) {
    return new Transition(Transition::move(static_cast<Edges>(edge)));
}
ELG_EXPORT Transition* elg_transition_slide() {
    return new Transition(Transition::slide());
}
ELG_EXPORT Transition* elg_transition_asymmetric(const Transition* insertion, const Transition* removal) {
    return new Transition(Transition::asymmetric(*insertion, *removal));
}
ELG_EXPORT Transition* elg_transition_combined(const Transition* a, const Transition* b) {
    return new Transition(a->combined(*b));
}
ELG_EXPORT ElgPose elg_transition_pose(const Transition* transition, float progress, bool removal, ElgSize view_size) {
    const Transition::Pose pose = transition->pose(progress, removal, size(view_size));
    return {pose.opacity, pose.scale, pose.x, pose.y};
}
ELG_EXPORT Transition* elg_transition_clone(const Transition* transition) {
    return new Transition(*transition);
}
ELG_EXPORT void elg_transition_free(Transition* transition) {
    delete transition;
}

// --- ABI layout (shim.h's structs and this file's) ---

ELG_EXPORT void elg_layouts_core(ElgStrSink out, void* ctx) {
    ELG_LAYOUT(ElgStr);
    ELG_LAYOUT(ElgBytes);
    ELG_LAYOUT(ElgClosure);
    ELG_LAYOUT(ElgColor);
    ELG_LAYOUT(ElgSize);
    ELG_LAYOUT(ElgPixelSize);
    ELG_LAYOUT(ElgPoint);
    ELG_LAYOUT(ElgRect);
    ELG_LAYOUT(ElgFont);
    ELG_LAYOUT(ElgGradientStop);
    ELG_LAYOUT(ElgShapeStyle);
    ELG_LAYOUT(ElgEdgeInsets);
    ELG_LAYOUT(ElgProp);
    ELG_LAYOUT(ElgProposedSize);
    ELG_LAYOUT(ElgOptLen);
    ELG_LAYOUT(ElgAlignment);
    ELG_LAYOUT(ElgKeyEvent);
    ELG_LAYOUT(ElgPointerEvent);
    ELG_LAYOUT(ElgScrollEvent);
    ELG_LAYOUT(ElgClickEvent);
    ELG_LAYOUT(ElgDragEvent);
    ELG_LAYOUT(ElgMagnifyEvent);
    ELG_LAYOUT(ElgRotateEvent);
    ELG_LAYOUT(ElgTextInputEvent);
    ELG_LAYOUT(ElgEventResult);
    ELG_LAYOUT(ElgFloatingPlacement);
    ELG_LAYOUT(ElgPopupMotion);
    ELG_LAYOUT(ElgHoverDelay);
    ELG_LAYOUT(ElgShadow);
    ELG_LAYOUT(ElgStrokeStyle);
    ELG_LAYOUT(ElgRunStyle);
    ELG_LAYOUT(ElgImageSource);
    ELG_LAYOUT(ElgWindowDesc);
    ELG_LAYOUT(ElgPose);
}
