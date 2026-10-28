// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// https://mozilla.org/MPL/2.0/.

// Extension points implemented in Rust: layouts, modifiers, event handlers. (Styles: styles.cpp.)

#include "shim.h"

using namespace elg;
using namespace elg_shim;

// --- Layout ---

struct ElgLayoutCall {
    int32_t op; // 0: size_that_fits (ret ElgSize*), 1: place_children, 2: spacer_axis (ret int32_t*: -1 none)
    ElgProposedSize proposal;
    ElgRect bounds;
    const LayoutChild* children;
    size_t count;
    LayoutCache* cache; // null: uncached
};

namespace {

class RustLayout final : public Layout {
public:
    explicit RustLayout(SharedFn f) : f_(std::move(f)) {}

    Size size_that_fits(ProposedSize proposal, std::span<const LayoutChild> children) const override {
        return size(proposal, {}, children, nullptr);
    }
    void place_children(Rect bounds, ProposedSize proposal, std::span<const LayoutChild> children) const override {
        place(bounds, proposal, children, nullptr);
    }
    std::optional<Axis> spacer_axis() const override {
        ElgLayoutCall call{2, {}, {}, nullptr, 0, nullptr};
        int32_t axis = -1;
        f_->call(&call, &axis);
        if (axis < 0) {
            return std::nullopt;
        }
        return static_cast<Axis>(axis);
    }
    Size cached_size_that_fits(ProposedSize proposal, std::span<const LayoutChild> children, LayoutCache& cache) const override {
        return size(proposal, {}, children, &cache);
    }
    void cached_place_children(Rect bounds, ProposedSize proposal, std::span<const LayoutChild> children,
                               LayoutCache& cache) const override {
        place(bounds, proposal, children, &cache);
    }

private:
    Size size(ProposedSize proposal, Rect, std::span<const LayoutChild> children, LayoutCache* cache) const {
        ElgLayoutCall call{0, c_proposed(proposal), {}, children.data(), children.size(), cache};
        ElgSize result{};
        f_->call(&call, &result);
        return {result.width, result.height};
    }
    void place(Rect bounds, ProposedSize proposal, std::span<const LayoutChild> children, LayoutCache* cache) const {
        ElgLayoutCall call{1, c_proposed(proposal), c_rect(bounds), children.data(), children.size(), cache};
        f_->call(&call, nullptr);
    }

    SharedFn f_;
};

} // namespace

ELG_EXPORT View* elg_rust_layout_view(ElgClosure layout, View** children, size_t count) {
    ELG_TRY
    const RustLayout rust_layout(std::make_shared<const RustFn>(layout));
    return out(rust_layout(take_all(children, count)));
    ELG_CATCH(nullptr)
}

// The Rust cache stored in `cache`, or null.
ELG_EXPORT void* elg_layout_cache_get(const LayoutCache* cache) {
    if (!cache->value) {
        return nullptr;
    }
    return static_cast<const RustFn*>(cache->value.get())->data();
}
ELG_EXPORT void elg_layout_cache_set(LayoutCache* cache, ElgClosure owner) {
    cache->value = std::shared_ptr<void>(std::make_shared<RustFn>(owner));
}
ELG_EXPORT bool elg_layout_cache_stale(const LayoutCache* cache) {
    return cache->stale;
}
ELG_EXPORT void elg_layout_cache_set_stale(LayoutCache* cache, bool stale) {
    cache->stale = stale;
}

ELG_EXPORT ElgSize elg_layout_child_size_that_fits(const LayoutChild* child, ElgProposedSize proposal) {
    ELG_TRY
    const Size s = child->size_that_fits(proposed(proposal));
    return {s.width, s.height};
    ELG_CATCH(ElgSize{})
}
ELG_EXPORT void elg_layout_child_place(const LayoutChild* child, ElgPoint position, ElgProposedSize proposal) {
    ELG_TRY
    child->place(point(position), proposed(proposal));
    ELG_CATCH_VOID
}
ELG_EXPORT float elg_layout_child_priority(const LayoutChild* child) {
    return child->layout_priority();
}
ELG_EXPORT bool elg_layout_child_is_spacer(const LayoutChild* child) {
    return child->is_spacer();
}

// --- Layout values set from Rust ---
//
// One C++ key carries every Rust key. Before a lookup the Rust side records which key it wants;
// each modifier's `matches` closure (ret: bool*) answers whether it holds that key.

namespace {

struct RustLayoutValue {
    const void* data = nullptr;
};

struct RustLayoutKey {
    using Value = RustLayoutValue;
    static RustLayoutValue default_value() { return {}; }
};

class RustLayoutValueModifier final : public LayoutValueModifierBase {
public:
    explicit RustLayoutValueModifier(SharedFn owner) : owner_(std::move(owner)), value_{owner_->data()} {}
    const void* value_for(const std::type_info& key) const noexcept override {
        if (key != typeid(RustLayoutKey)) {
            return nullptr;
        }
        bool matches = false;
        try {
            owner_->call(nullptr, &matches);
        } catch (...) {
            return nullptr;
        }
        return matches ? &value_ : nullptr;
    }

private:
    SharedFn owner_;
    RustLayoutValue value_;
};

} // namespace

ELG_EXPORT View* elg_mod_rust_layout_value(const View* v, ElgClosure owner) {
    ELG_TRY
    return out(v->modifier(RustLayoutValueModifier(std::make_shared<const RustFn>(owner))));
    ELG_CATCH(nullptr)
}

// The data of the matching Rust layout value on `child`, or null.
ELG_EXPORT const void* elg_layout_child_rust_value(const LayoutChild* child) {
    ELG_TRY
    return child->value<RustLayoutKey>().data;
    ELG_CATCH(nullptr)
}

// --- Modifiers ---

namespace {

class RustViewModifier final : public ViewModifier {
public:
    explicit RustViewModifier(SharedFn f) : f_(std::move(f)) {}
    // args: View* (owned by the Rust side), ret: View*.
    View body(View content) const override {
        View result;
        f_->call(new View(std::move(content)), &result);
        return result;
    }

private:
    SharedFn f_;
};

} // namespace

ELG_EXPORT View* elg_mod_rust_view_modifier(const View* v, ElgClosure body) {
    ELG_TRY
    return out(v->modifier(RustViewModifier(std::make_shared<const RustFn>(body))));
    ELG_CATCH(nullptr)
}

struct ElgLayoutModifierCall {
    int32_t op; // 0: size_that_fits (ret ElgSize*), 1: place, 2: layout_priority (ret ElgOptLen*)
    ElgProposedSize proposal;
    ElgRect bounds;
    const LayoutChild* content;
};

namespace {

class RustLayoutModifier final : public LayoutModifier {
public:
    explicit RustLayoutModifier(SharedFn f) : f_(std::move(f)) {}

    Size size_that_fits(ProposedSize proposal, const LayoutChild& content) const override {
        ElgLayoutModifierCall call{0, c_proposed(proposal), {}, &content};
        ElgSize result{};
        f_->call(&call, &result);
        return {result.width, result.height};
    }
    void place(Rect bounds, ProposedSize proposal, const LayoutChild& content) const override {
        ElgLayoutModifierCall call{1, c_proposed(proposal), c_rect(bounds), &content};
        f_->call(&call, nullptr);
    }
    std::optional<float> layout_priority() const override {
        ElgLayoutModifierCall call{2, {}, {}, nullptr};
        ElgOptLen result{};
        f_->call(&call, &result);
        if (!result.has) {
            return std::nullopt;
        }
        return result.value;
    }

private:
    SharedFn f_;
};

class RustDecorationModifier final : public DecorationModifier {
public:
    RustDecorationModifier(SharedFn f, Placement placement, Alignment alignment)
        : f_(std::move(f)), placement_(placement), alignment_(alignment) {}
    View decoration() const override {
        View result;
        f_->call(nullptr, &result);
        return result;
    }
    Placement placement() const override { return placement_; }
    Alignment alignment() const override { return alignment_; }

private:
    SharedFn f_;
    Placement placement_;
    Alignment alignment_;
};

class RustEnvironmentModifier final : public EnvironmentModifier {
public:
    explicit RustEnvironmentModifier(SharedFn f) : f_(std::move(f)) {}
    // args: EnvironmentValues*.
    void apply(EnvironmentValues& values) const override { f_->call(&values, nullptr); }

private:
    SharedFn f_;
};

class RustEffectModifier final : public EffectModifier {
public:
    explicit RustEffectModifier(SharedFn f) : f_(std::move(f)) {}
    // args: Effects*.
    void apply(Effects& effects) const override { f_->call(&effects, nullptr); }

private:
    SharedFn f_;
};

} // namespace

ELG_EXPORT View* elg_mod_rust_layout_modifier(const View* v, ElgClosure layout) {
    ELG_TRY
    return out(v->modifier(RustLayoutModifier(std::make_shared<const RustFn>(layout))));
    ELG_CATCH(nullptr)
}
// placement: DecorationModifier::Placement (behind, in_front, mask).
ELG_EXPORT View* elg_mod_rust_decoration_modifier(const View* v, ElgClosure decoration, int32_t placement, ElgAlignment a) {
    ELG_TRY
    return out(v->modifier(RustDecorationModifier(std::make_shared<const RustFn>(decoration),
                                                  static_cast<DecorationModifier::Placement>(placement), alignment(a))));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_rust_environment_modifier(const View* v, ElgClosure apply) {
    ELG_TRY
    return out(v->modifier(RustEnvironmentModifier(std::make_shared<const RustFn>(apply))));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_rust_effect_modifier(const View* v, ElgClosure apply) {
    ELG_TRY
    return out(v->modifier(RustEffectModifier(std::make_shared<const RustFn>(apply))));
    ELG_CATCH(nullptr)
}

// Effects, for EffectModifier::apply.
ELG_EXPORT void elg_effects_set_clip_shape(Effects* e, const Shape* shape) {
    if (shape) {
        e->clip_shape = *shape;
    } else {
        e->clip_shape.reset();
    }
}
ELG_EXPORT void elg_effects_set_opacity(Effects* e, ElgProp opacity) {
    ELG_TRY
    e->opacity = prop<float>(opacity);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_effects_set_shadow(Effects* e, const ElgShadow* s) {
    if (s) {
        e->shadow = shadow(*s);
    } else {
        e->shadow.reset();
    }
}
ELG_EXPORT void elg_effects_set_offset(Effects* e, ElgProp x, ElgProp y) {
    ELG_TRY
    e->offset_x = prop<float>(x);
    e->offset_y = prop<float>(y);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_effects_set_z_index(Effects* e, const ElgProp* z) {
    ELG_TRY
    if (z) {
        e->z_index = prop<float>(*z);
    } else {
        e->z_index.reset();
    }
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_effects_set_content_shape(Effects* e, const Shape* shape) {
    if (shape) {
        e->content_shape = *shape;
    } else {
        e->content_shape.reset();
    }
}
ELG_EXPORT void elg_effects_set_inner_shadow(Effects* e, const ElgShadow* s) {
    if (s) {
        e->inner_shadow = shadow(*s);
    } else {
        e->inner_shadow.reset();
    }
}
ELG_EXPORT void elg_effects_set_blur(Effects* e, const ElgProp* radius, bool opaque) {
    ELG_TRY
    if (radius) {
        e->blur = prop<float>(*radius);
    } else {
        e->blur.reset();
    }
    e->blur_opaque = opaque;
    ELG_CATCH_VOID
}
// kind < 0: none.
ELG_EXPORT void elg_effects_set_color_filter(Effects* e, int32_t kind, ElgProp amount, ElgProp c) {
    ELG_TRY
    if (kind < 0) {
        e->color_filter.reset();
        return;
    }
    ColorFilter filter;
    filter.kind = static_cast<ColorFilter::Kind>(kind);
    filter.amount = prop<float>(amount);
    filter.color = prop<Color>(c);
    e->color_filter = std::move(filter);
    ELG_CATCH_VOID
}
ELG_EXPORT int32_t elg_effects_blend_mode(const Effects* e) {
    return static_cast<int32_t>(e->blend_mode);
}
ELG_EXPORT void elg_effects_set_blend_mode(Effects* e, int32_t mode) {
    e->blend_mode = static_cast<BlendMode>(mode);
}
ELG_EXPORT bool elg_effects_compositing_group(const Effects* e) {
    return e->compositing_group;
}
ELG_EXPORT void elg_effects_set_compositing_group(Effects* e, bool group) {
    e->compositing_group = group;
}
ELG_EXPORT void elg_effects_set_rotation(Effects* e, const ElgProp* degrees) {
    ELG_TRY
    if (degrees) {
        e->rotation = prop<float>(*degrees);
    } else {
        e->rotation.reset();
    }
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_effects_set_scale(Effects* e, const ElgProp* x, const ElgProp* y) {
    ELG_TRY
    if (x) {
        e->scale_x = prop<float>(*x);
    } else {
        e->scale_x.reset();
    }
    if (y) {
        e->scale_y = prop<float>(*y);
    } else {
        e->scale_y.reset();
    }
    ELG_CATCH_VOID
}
ELG_EXPORT ElgPoint elg_effects_anchor(const Effects* e) {
    return {e->anchor.x, e->anchor.y};
}
ELG_EXPORT void elg_effects_set_anchor(Effects* e, ElgPoint anchor) {
    e->anchor = unit_point(anchor);
}
ELG_EXPORT bool elg_effects_allows_hit_testing(const Effects* e) {
    return e->allows_hit_testing;
}
ELG_EXPORT void elg_effects_set_allows_hit_testing(Effects* e, bool enabled) {
    e->allows_hit_testing = enabled;
}

// --- Event handlers ---

struct ElgEventCall {
    // 0 pointer, 1 hover, 2 scroll, 3 key, 4 focusable (ret bool*), 5 focus, 6 accepts_text_input
    // (ret bool*), 7 text_input, 8 hover_delay (ret ElgOptHoverDelay*), 9 covers_frame (ret bool*),
    // 10 pointer_style (ret int32_t*), 11 magnify, 12 rotate, 13 two_finger_double_tap.
    // The others: ret ElgEventResult*. `frame`: pointer and scroll.
    int32_t op;
    ElgPointerEvent pointer;
    ElgRect frame;
    bool flag; // hover: inside; focus: focused
    ElgScrollEvent scroll;
    ElgKeyEvent key;
    ElgTextInputEvent text;
    ElgPoint location; // pointer_style, two_finger_double_tap
    ElgMagnifyEvent magnify;
    ElgRotateEvent rotate;
};

struct ElgOptHoverDelay {
    bool has;
    ElgHoverDelay delay;
};

namespace {

uint32_t mods(KeyModifiers m) {
    return static_cast<uint32_t>(m);
}

class RustEventHandler final : public EventHandler {
public:
    explicit RustEventHandler(SharedFn f) : f_(std::move(f)) {}

    EventResult pointer(const PointerEvent& event, Rect frame) override {
        ElgEventCall call{};
        call.op = 0;
        call.pointer = c_pointer_event(event);
        call.frame = c_rect(frame);
        return result(call);
    }
    EventResult hover(bool inside) override {
        ElgEventCall call{};
        call.op = 1;
        call.flag = inside;
        return result(call);
    }
    PointerStyle pointer_style(Point location) const override {
        ElgEventCall call{};
        call.op = 10;
        call.location = {location.x, location.y};
        int32_t style = 0;
        f_->call(&call, &style);
        return static_cast<PointerStyle>(style);
    }
    EventResult scroll(const ScrollEvent& event, Rect frame) override {
        ElgEventCall call{};
        call.op = 2;
        call.scroll = c_scroll_event(event);
        call.frame = c_rect(frame);
        return result(call);
    }
    EventResult magnify(const MagnifyEvent& event) override {
        ElgEventCall call{};
        call.op = 11;
        call.magnify = c_magnify_event(event);
        return result(call);
    }
    EventResult rotate(const RotateEvent& event) override {
        ElgEventCall call{};
        call.op = 12;
        call.rotate = c_rotate_event(event);
        return result(call);
    }
    EventResult two_finger_double_tap(Point location) override {
        ElgEventCall call{};
        call.op = 13;
        call.location = {location.x, location.y};
        return result(call);
    }
    EventResult key(const KeyEvent& event) override {
        ElgEventCall call{};
        call.op = 3;
        call.key = c_key_event(event);
        return result(call);
    }
    bool focusable() const override { return flag(4); }
    EventResult focus(bool focused) override {
        ElgEventCall call{};
        call.op = 5;
        call.flag = focused;
        return result(call);
    }
    bool accepts_text_input() const override { return flag(6); }
    EventResult text_input(const TextInputEvent& event) override {
        ElgEventCall call{};
        call.op = 7;
        call.text = {static_cast<int32_t>(event.kind), c_str(event.text), event.cursor};
        return result(call);
    }
    std::optional<HoverDelay> hover_delay() const override {
        ElgEventCall call{};
        call.op = 8;
        ElgOptHoverDelay delay{};
        f_->call(&call, &delay);
        if (!delay.has) {
            return std::nullopt;
        }
        return elg_shim::hover_delay(delay.delay);
    }
    bool covers_frame() const override { return flag(9); }

private:
    EventResult result(ElgEventCall& call) {
        ElgEventResult r{};
        f_->call(&call, &r);
        return {r.consumed, r.redraw, r.cancellable};
    }
    bool flag(int32_t op) const {
        ElgEventCall call{};
        call.op = op;
        bool value = false;
        f_->call(&call, &value);
        return value;
    }

    SharedFn f_;
};

class RustEventModifier final : public EventModifier {
public:
    explicit RustEventModifier(SharedFn f) : f_(std::move(f)) {}
    // ret: ElgClosure* (the new handler).
    std::unique_ptr<EventHandler> make_handler() const override {
        ElgClosure handler{};
        f_->call(nullptr, &handler);
        return std::make_unique<RustEventHandler>(std::make_shared<const RustFn>(handler));
    }

private:
    SharedFn f_;
};

} // namespace

ELG_EXPORT View* elg_mod_rust_event_modifier(const View* v, ElgClosure make_handler) {
    ELG_TRY
    return out(v->modifier(RustEventModifier(std::make_shared<const RustFn>(make_handler))));
    ELG_CATCH(nullptr)
}

// --- ABI layout ---

ELG_EXPORT void elg_layouts_ext(ElgStrSink out, void* ctx) {
    ELG_LAYOUT(ElgLayoutCall);
    ELG_LAYOUT(ElgLayoutModifierCall);
    ELG_LAYOUT(ElgEventCall);
    ELG_LAYOUT(ElgOptHoverDelay);
}
