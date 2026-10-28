// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// https://mozilla.org/MPL/2.0/.

// The C ABI between Elegance (C++23) and the Rust bindings. Every function the Rust side calls is
// `extern "C"` and noexcept in practice: C++ exceptions are caught and recorded for
// elg_take_error(). Rust closures arrive as ElgClosure; a closure that panicked returns false,
// and the shim turns that into a RustPanic exception so Elegance unwinds as it does for its own
// errors.
//
// Ownership: an `elg::View*` (and Text*, Shape*, ...) argument named `take_*` or passed in an
// array is consumed. `const T*` arguments are borrowed. Returned pointers are owned by the caller.

#ifndef ELG_RUST_SHIM_H
#define ELG_RUST_SHIM_H

#include <elg/elegance.h>

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <functional>
#include <memory>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

// ELG_SHIM_SHARED: the shim and Elegance built into one shared library (native/CMakeLists.txt, the
// prebuilt library), which exports just these functions and ELG_API.
#if defined(ELG_SHIM_SHARED) && defined(_WIN32)
#define ELG_EXPORT extern "C" __declspec(dllexport)
#elif defined(ELG_SHIM_SHARED)
#define ELG_EXPORT extern "C" __attribute__((visibility("default")))
#else
#define ELG_EXPORT extern "C"
#endif

extern "C" {

struct ElgStr {
    const char* ptr;
    size_t len;
};

struct ElgBytes {
    const unsigned char* ptr;
    size_t len;
};

// Calls a Rust closure. `args` and `ret` point at values whose types depend on the call site.
// Returns false if the closure panicked.
typedef bool (*ElgCallFn)(void* data, void* args, void* ret);
typedef void (*ElgDropFn)(void* data);

// A Rust closure. `call` null: no closure (optional callbacks). `drop` null: borrowed.
struct ElgClosure {
    void* data;
    ElgCallFn call;
    ElgDropFn drop;
};

typedef void (*ElgStrSink)(void* ctx, const char* ptr, size_t len);

struct ElgColor {
    float r, g, b, a;
};

struct ElgSize {
    float width, height;
};

struct ElgPixelSize {
    uint32_t width, height;
};

// on_size_change args: ElgPixelSize*. Callback ownership is transferred to the view.
ELG_EXPORT elg::View* elg_mod_on_size_change(const elg::View* v, ElgClosure on_size_change);

struct ElgPoint {
    float x, y;
};

struct ElgRect {
    float x, y, width, height;
};

struct ElgFont {
    ElgStr family;
    float size;
    int32_t weight;
    bool italic;
};

struct ElgGradientStop {
    ElgColor color;
    float location;
};

// kind 0: color; 1: linear gradient from (x0, y0) to (x1, y1); 2: radial gradient around (x0, y0)
// from radius r0 to r1; 3: elliptical gradient around (x0, y0) from radius fraction r0 to r1;
// 4: angular gradient around (x0, y0) from angle r0 to r1 (degrees).
struct ElgShapeStyle {
    int32_t kind;
    ElgColor color;
    const ElgGradientStop* stops;
    size_t stop_count;
    float x0, y0, x1, y1;
    float r0, r1;
};

struct ElgEdgeInsets {
    float top, leading, bottom, trailing;
};

// A reactive argument. kind 0: `value` points at the static value (its C form); 1: `fn` returns
// it (ret: a C++ T*); 2: `fn` takes the environment (args: const EnvironmentValues*); 3: the
// theme color `theme` (Color and ShapeStyle props).
struct ElgProp {
    int32_t kind;
    int32_t theme;
    const void* value;
    ElgClosure fn;
};

struct ElgProposedSize {
    bool has_width;
    float width;
    bool has_height;
    float height;
};

struct ElgOptLen {
    bool has;
    float value;
};

struct ElgAlignment {
    int32_t horizontal;
    int32_t vertical;
};

struct ElgKeyEvent {
    int32_t kind;
    int32_t key;
    uint32_t modifiers;
    bool repeat;
};

struct ElgPointerEvent {
    int32_t kind;
    ElgPoint location;
    int32_t button;
    int32_t click_count;
    uint32_t modifiers;
    ElgPoint window_location;
};

struct ElgScrollEvent {
    ElgPoint location;
    ElgPoint window_location;
    float dx, dy;
    bool precise;
    uint32_t modifiers;
    int32_t phase;
    bool momentum;
    double time;
};

struct ElgClickEvent {
    ElgPoint location;
    ElgPoint window_location;
    int32_t button;
    uint32_t modifiers;
    int32_t click_count;
};

struct ElgDragEvent {
    ElgPoint location;
    ElgPoint start_location;
    ElgPoint translation;
    ElgPoint velocity;
    int32_t button;
    uint32_t modifiers;
};

struct ElgMagnifyEvent {
    ElgPoint location;
    float scale;
    float delta;
    int32_t phase;
};

struct ElgRotateEvent {
    ElgPoint location;
    float degrees;
    float delta;
    int32_t phase;
};

struct ElgTextInputEvent {
    int32_t kind;
    ElgStr text;
    size_t cursor;
};

struct ElgEventResult {
    bool consumed;
    bool redraw;
    bool cancellable;
};

// elg::FloatingPlacement: edge (FloatingEdge), alignment (FloatingAlignment), and an optional
// position instead of an edge.
struct ElgFloatingPlacement {
    int32_t edge;
    int32_t alignment;
    float spacing;
    ElgPoint offset;
    bool has_position;
    ElgPoint position;
};

// An optional elg::PopupMotion: `has` false is nullopt (the style's); null transition or
// animation is PopupMotion::none(). Borrowed.
struct ElgPopupMotion {
    bool has;
    const elg::Transition* transition;
    const elg::Animation* animation;
};

struct ElgHoverDelay {
    double enter;
    double exit;
};

struct ElgShadow {
    ElgColor color;
    float radius, x, y;
    float spread;
};

struct ElgStrokeStyle {
    float width;
    int32_t cap;
    int32_t join;
    float miter_limit;
    const float* dash;
    size_t dash_count;
    float dash_phase;
};

struct ElgRunStyle {
    bool has_weight;
    int32_t weight;
    bool italic;
    ElgStr family;
    bool has_color;
    ElgColor color;
    bool has_background;
    ElgColor background;
    bool underline;
    bool strikethrough;
    bool keep_together;
};

struct ElgImageSource {
    ElgStr system_name;
    ElgStr file;
    ElgBytes data;
    float scale;
};

} // extern "C"

namespace elg_shim {

// Thrown through Elegance when a Rust closure panicked; the panic payload stays on the Rust side.
struct RustPanic final : std::exception {
    const char* what() const noexcept override { return "a Rust callback panicked"; }
};

// Owns a Rust closure.
class RustFn {
public:
    explicit RustFn(ElgClosure closure) noexcept : closure_(closure) {}
    ~RustFn() {
        if (closure_.drop) {
            closure_.drop(closure_.data);
        }
    }
    RustFn(const RustFn&) = delete;
    RustFn& operator=(const RustFn&) = delete;

    void call(void* args, void* ret) const {
        if (!closure_.call(closure_.data, args, ret)) {
            throw RustPanic();
        }
    }
    void* data() const noexcept { return closure_.data; }

private:
    ElgClosure closure_;
};

using SharedFn = std::shared_ptr<const RustFn>;

inline SharedFn share(ElgClosure closure) {
    if (!closure.call) {
        if (closure.drop) {
            closure.drop(closure.data);
        }
        return nullptr;
    }
    return std::make_shared<const RustFn>(closure);
}

// Calls a borrowed closure (one that lives for the duration of a call).
inline void call_now(const ElgClosure& closure, void* args, void* ret) {
    if (!closure.call(closure.data, args, ret)) {
        throw RustPanic();
    }
}

inline std::string str(ElgStr s) {
    return s.ptr ? std::string(s.ptr, s.len) : std::string();
}

inline std::string_view view(ElgStr s) {
    return s.ptr ? std::string_view(s.ptr, s.len) : std::string_view();
}

inline ElgStr c_str(const std::string& s) {
    return {s.data(), s.size()};
}

inline std::filesystem::path path(ElgStr s) {
    const std::string_view text = view(s);
    return std::filesystem::path(std::u8string_view(reinterpret_cast<const char8_t*>(text.data()), text.size()));
}

inline std::string utf8(const std::filesystem::path& p) {
    const std::u8string text = p.u8string();
    return std::string(reinterpret_cast<const char*>(text.data()), text.size());
}

inline std::span<const unsigned char> bytes(ElgBytes b) {
    return b.ptr ? std::span<const unsigned char>(b.ptr, b.len) : std::span<const unsigned char>();
}

inline void sink(ElgStrSink out, void* ctx, const std::string& s) {
    out(ctx, s.data(), s.size());
}

// --- std::function adapters over Rust closures ---

inline std::function<void()> action(ElgClosure closure) {
    SharedFn f = share(closure);
    if (!f) {
        return {};
    }
    return [f] { f->call(nullptr, nullptr); };
}

// A closure returning R, written by the Rust side into a default-constructed R.
template <class R>
std::function<R()> producer(ElgClosure closure) {
    SharedFn f = share(closure);
    if (!f) {
        return {};
    }
    return [f] {
        R result{};
        f->call(nullptr, &result);
        return result;
    };
}

// A closure taking one argument; Rust receives a pointer to `A` (a C type).
template <class A>
std::function<void(A)> consumer(ElgClosure closure) {
    SharedFn f = share(closure);
    if (!f) {
        return {};
    }
    return [f](A value) { f->call(&value, nullptr); };
}

inline std::function<elg::View()> view_builder(ElgClosure closure) {
    return producer<elg::View>(closure);
}

// --- Conversions between C structs and Elegance values ---

inline elg::Color color(ElgColor c) {
    return {c.r, c.g, c.b, c.a};
}

inline ElgColor c_color(elg::Color c) {
    return {c.r, c.g, c.b, c.a};
}

inline elg::Size size(ElgSize s) {
    return {s.width, s.height};
}

inline elg::Point point(ElgPoint p) {
    return {p.x, p.y};
}

inline elg::Rect rect(ElgRect r) {
    return {r.x, r.y, r.width, r.height};
}

inline ElgRect c_rect(elg::Rect r) {
    return {r.x, r.y, r.width, r.height};
}

inline elg::ProposedSize proposed(ElgProposedSize p) {
    elg::ProposedSize result;
    if (p.has_width) {
        result.width = p.width;
    }
    if (p.has_height) {
        result.height = p.height;
    }
    return result;
}

inline ElgProposedSize c_proposed(elg::ProposedSize p) {
    return {p.width.has_value(), p.width.value_or(0), p.height.has_value(), p.height.value_or(0)};
}

inline elg::OptionalLength length(ElgOptLen l) {
    return l.has ? elg::OptionalLength(l.value) : elg::OptionalLength();
}

inline elg::Alignment alignment(ElgAlignment a) {
    return {static_cast<elg::HorizontalAlignment>(a.horizontal), static_cast<elg::VerticalAlignment>(a.vertical)};
}

inline ElgAlignment c_alignment(elg::Alignment a) {
    return {static_cast<int32_t>(a.horizontal), static_cast<int32_t>(a.vertical)};
}

inline elg::Font font(const ElgFont& f) {
    const auto weight = static_cast<elg::Weight>(f.weight);
    elg::Font result = f.family.len ? elg::Font::custom(str(f.family), f.size, weight) : elg::Font::system(f.size, weight);
    return result.italic(f.italic);
}

inline std::vector<elg::GradientStop> stops(const ElgShapeStyle& s) {
    std::vector<elg::GradientStop> result;
    result.reserve(s.stop_count);
    for (size_t i = 0; i < s.stop_count; ++i) {
        result.push_back({color(s.stops[i].color), s.stops[i].location});
    }
    return result;
}

inline elg::ShapeStyle shape_style(const ElgShapeStyle& s) {
    switch (s.kind) {
    case 1: return elg::LinearGradient(stops(s), elg::UnitPoint{s.x0, s.y0}, elg::UnitPoint{s.x1, s.y1});
    case 2: return elg::RadialGradient(stops(s), elg::UnitPoint{s.x0, s.y0}, s.r0, s.r1);
    case 3: return elg::EllipticalGradient(stops(s), elg::UnitPoint{s.x0, s.y0}, s.r0, s.r1);
    case 4: return elg::AngularGradient(stops(s), elg::UnitPoint{s.x0, s.y0}, s.r0, s.r1);
    default: return color(s.color);
    }
}

inline elg::EdgeInsets edge_insets(ElgEdgeInsets i) {
    return {i.top, i.leading, i.bottom, i.trailing};
}

inline elg::FloatingPlacement placement(const ElgFloatingPlacement& p) {
    elg::FloatingPlacement result;
    result.edge = static_cast<elg::FloatingEdge>(p.edge);
    result.alignment = static_cast<elg::FloatingAlignment>(p.alignment);
    result.spacing = p.spacing;
    result.offset = point(p.offset);
    if (p.has_position) {
        result.position = point(p.position);
    }
    return result;
}

inline std::optional<elg::PopupMotion> motion(const ElgPopupMotion& m) {
    if (!m.has) {
        return std::nullopt;
    }
    if (!m.transition || !m.animation) {
        return elg::PopupMotion::none();
    }
    return elg::PopupMotion(*m.transition, *m.animation);
}

inline std::optional<elg::Animation> animation(const elg::Animation* a) {
    return a ? std::optional<elg::Animation>(*a) : std::nullopt;
}

inline elg::UnitPoint unit_point(ElgPoint p) {
    return {p.x, p.y};
}

inline elg::HoverDelay hover_delay(ElgHoverDelay d) {
    return {d.enter, d.exit};
}

inline elg::Shadow shadow(ElgShadow s) {
    return {color(s.color), s.radius, s.x, s.y, s.spread};
}

inline elg::StrokeStyle stroke_style(const ElgStrokeStyle& s) {
    elg::StrokeStyle result;
    result.width = s.width;
    result.cap = static_cast<elg::LineCap>(s.cap);
    result.join = static_cast<elg::LineJoin>(s.join);
    result.miter_limit = s.miter_limit;
    if (s.dash) {
        result.dash.assign(s.dash, s.dash + s.dash_count);
    }
    result.dash_phase = s.dash_phase;
    return result;
}

inline elg::RunStyle run_style(const ElgRunStyle& s) {
    elg::RunStyle result;
    if (s.has_weight) {
        result.weight = static_cast<elg::Weight>(s.weight);
    }
    result.italic = s.italic;
    result.family = str(s.family);
    if (s.has_color) {
        result.color = color(s.color);
    }
    if (s.has_background) {
        result.background = color(s.background);
    }
    result.underline = s.underline;
    result.strikethrough = s.strikethrough;
    result.keep_together = s.keep_together;
    return result;
}

inline elg::ImageSource image_source(const ElgImageSource& s) {
    elg::ImageSource result;
    result.system_name = str(s.system_name);
    result.file = path(s.file);
    result.data = bytes(s.data);
    result.scale = s.scale;
    return result;
}

inline ElgKeyEvent c_key_event(const elg::KeyEvent& e) {
    return {static_cast<int32_t>(e.kind), static_cast<int32_t>(e.key), static_cast<uint32_t>(e.modifiers), e.repeat};
}

inline ElgPoint c_point(elg::Point p) {
    return {p.x, p.y};
}

inline ElgPointerEvent c_pointer_event(const elg::PointerEvent& e) {
    return {static_cast<int32_t>(e.kind), c_point(e.location), static_cast<int32_t>(e.button), e.click_count,
            static_cast<uint32_t>(e.modifiers), c_point(e.window_location)};
}

inline ElgScrollEvent c_scroll_event(const elg::ScrollEvent& e) {
    return {c_point(e.location), c_point(e.window_location), e.dx, e.dy, e.precise, static_cast<uint32_t>(e.modifiers),
            static_cast<int32_t>(e.phase), e.momentum, e.time};
}

inline ElgClickEvent c_click_event(const elg::ClickEvent& e) {
    return {c_point(e.location), c_point(e.window_location), static_cast<int32_t>(e.button),
            static_cast<uint32_t>(e.modifiers), e.click_count};
}

inline ElgDragEvent c_drag_event(const elg::DragEvent& e) {
    return {c_point(e.location), c_point(e.start_location), c_point(e.translation), c_point(e.velocity),
            static_cast<int32_t>(e.button), static_cast<uint32_t>(e.modifiers)};
}

inline ElgMagnifyEvent c_magnify_event(const elg::MagnifyEvent& e) {
    return {{e.location.x, e.location.y}, e.scale, e.delta, static_cast<int32_t>(e.phase)};
}

inline ElgRotateEvent c_rotate_event(const elg::RotateEvent& e) {
    return {{e.location.x, e.location.y}, e.degrees, e.delta, static_cast<int32_t>(e.phase)};
}

// The C form of each prop value type, for kind 0 (static) props.
template <class T>
struct PropValue;

template <>
struct PropValue<float> {
    static float from(const void* v) { return *static_cast<const float*>(v); }
};
template <>
struct PropValue<bool> {
    static bool from(const void* v) { return *static_cast<const bool*>(v); }
};
template <>
struct PropValue<int> {
    static int from(const void* v) { return *static_cast<const int32_t*>(v); }
};
template <>
struct PropValue<double> {
    static double from(const void* v) { return *static_cast<const double*>(v); }
};
template <>
struct PropValue<elg::Weight> {
    static elg::Weight from(const void* v) { return static_cast<elg::Weight>(*static_cast<const int32_t*>(v)); }
};
template <>
struct PropValue<elg::EdgeInsets> {
    static elg::EdgeInsets from(const void* v) { return edge_insets(*static_cast<const ElgEdgeInsets*>(v)); }
};
// Empty: has false.
template <>
struct PropValue<std::optional<float>> {
    static std::optional<float> from(const void* v) {
        const auto& o = *static_cast<const ElgOptLen*>(v);
        return o.has ? std::optional<float>(o.value) : std::nullopt;
    }
};
template <>
struct PropValue<std::string> {
    static std::string from(const void* v) { return str(*static_cast<const ElgStr*>(v)); }
};
template <>
struct PropValue<elg::Color> {
    static elg::Color from(const void* v) { return color(*static_cast<const ElgColor*>(v)); }
};
template <>
struct PropValue<elg::Size> {
    static elg::Size from(const void* v) { return size(*static_cast<const ElgSize*>(v)); }
};
template <>
struct PropValue<elg::Font> {
    static elg::Font from(const void* v) { return font(*static_cast<const ElgFont*>(v)); }
};
template <>
struct PropValue<elg::ShapeStyle> {
    static elg::ShapeStyle from(const void* v) { return shape_style(*static_cast<const ElgShapeStyle*>(v)); }
};
template <>
struct PropValue<elg::ColorScheme> {
    static elg::ColorScheme from(const void* v) { return static_cast<elg::ColorScheme>(*static_cast<const int32_t*>(v)); }
};
template <>
struct PropValue<elg::PointerStyle> {
    static elg::PointerStyle from(const void* v) { return static_cast<elg::PointerStyle>(*static_cast<const int32_t*>(v)); }
};
template <>
struct PropValue<elg::Theme> {
    static elg::Theme from(const void* v) { return *static_cast<const elg::Theme*>(v); }
};
template <>
struct PropValue<elg::HoverDelay> {
    static elg::HoverDelay from(const void* v) { return hover_delay(*static_cast<const ElgHoverDelay*>(v)); }
};

// ShapeStyle has no default constructor; reactive closures write into one made from a color.
template <class T>
T blank() {
    if constexpr (std::is_same_v<T, elg::ShapeStyle>) {
        return elg::ShapeStyle(elg::Color{});
    } else if constexpr (std::is_same_v<T, elg::Font>) {
        return elg::Font::body;
    } else {
        return T{};
    }
}

template <class T>
elg::Prop<T> prop(const ElgProp& p) {
    switch (p.kind) {
    case 1: {
        SharedFn f = share(p.fn);
        return elg::Prop<T>([f] {
            T result = blank<T>();
            f->call(nullptr, &result);
            return result;
        });
    }
    case 2: {
        SharedFn f = share(p.fn);
        return elg::Prop<T>([f](const elg::EnvironmentValues& environment) {
            T result = blank<T>();
            f->call(const_cast<elg::EnvironmentValues*>(&environment), &result);
            return result;
        });
    }
    case 3:
        if constexpr (std::is_convertible_v<elg::Color, T>) {
            return elg::Prop<T>(static_cast<elg::ThemeColor>(p.theme));
        } else {
            return elg::Prop<T>(blank<T>());
        }
    default: return elg::Prop<T>(PropValue<T>::from(p.value));
    }
}

// --- Views across the boundary ---

inline elg::View* out(elg::View view) {
    return new elg::View(std::move(view));
}

inline elg::View take(elg::View* view) {
    elg::View result = std::move(*view);
    delete view;
    return result;
}

inline std::vector<elg::View> take_all(elg::View** views, size_t count) {
    std::vector<elg::View> result;
    result.reserve(count);
    for (size_t i = 0; i < count; ++i) {
        result.push_back(take(views[i]));
    }
    return result;
}

// A Rust component: its body runs each time the view is mounted.
struct RustComponent {
    SharedFn body_fn;
    elg::View body() const {
        elg::View result;
        body_fn->call(nullptr, &result);
        return result;
    }
};

// --- Errors ---

// Records the current exception for elg_take_error().
void capture_error() noexcept;

// --- ABI layout ---

// One line, "name size align", of an elg_layouts_* function: elegance-sys's layout test compares
// each C struct with its Rust declaration.
inline void layout(ElgStrSink out, void* ctx, const char* name, size_t size, size_t align) {
    sink(out, ctx, std::string(name) + ' ' + std::to_string(size) + ' ' + std::to_string(align) + '\n');
}

} // namespace elg_shim

#define ELG_LAYOUT(T) ::elg_shim::layout(out, ctx, #T, sizeof(T), alignof(T))

#define ELG_TRY try {
#define ELG_CATCH(fallback)               \
    }                                     \
    catch (...) {                         \
        ::elg_shim::capture_error();      \
        return fallback;                  \
    }
#define ELG_CATCH_VOID                    \
    }                                     \
    catch (...) {                         \
        ::elg_shim::capture_error();      \
    }

#endif // ELG_RUST_SHIM_H
