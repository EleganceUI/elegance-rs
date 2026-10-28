// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// https://mozilla.org/MPL/2.0/.

// Styles: the built-in ones, styles implemented in Rust, their configurations, and the modifiers
// that put them in the environment.
//
// A style crosses as `std::shared_ptr<const XStyle>*`, as the environment holds it. A Rust style's
// make_body gets a heap copy of the configuration (args, owned by the Rust side) and writes the
// body into ret (View*). Configuration accessors take `which`, an index into the configuration's
// views, states (Signal<bool>, tracked reads) or actions, in the order the header comments list.

#include "shim.h"

using namespace elg;
using namespace elg_shim;

namespace {

// Calls a Rust part builder with a copy of the configuration.
template <class Config>
View call(const RustFn& f, const Config& config) {
    View result;
    f.call(new Config(config), &result);
    return result;
}

// make_body only.
template <class Style, class Config>
class RustStyle final : public Style {
public:
    explicit RustStyle(SharedFn f) : f_(std::move(f)) {}
    View make_body(const Config& config) const override { return call(*f_, config); }

private:
    SharedFn f_;
};

// make_body and animation() (toggles, check boxes).
template <class Style, class Config>
class RustAnimatedStyle final : public Style {
public:
    RustAnimatedStyle(SharedFn f, std::optional<Animation> animation) : f_(std::move(f)), animation_(std::move(animation)) {}
    View make_body(const Config& config) const override { return call(*f_, config); }
    std::optional<Animation> animation() const override { return animation_; }

private:
    SharedFn f_;
    std::optional<Animation> animation_;
};

// make_body and motion() (popovers, tooltips).
template <class Style, class Config>
class RustPopupStyle final : public Style {
public:
    RustPopupStyle(SharedFn f, std::optional<PopupMotion> motion) : f_(std::move(f)), motion_(std::move(motion)) {}
    View make_body(const Config& config) const override { return call(*f_, config); }
    PopupMotion motion() const override { return motion_ ? *motion_ : Style::motion(); }

private:
    SharedFn f_;
    std::optional<PopupMotion> motion_;
};

class RustSliderStyle final : public SliderStyle {
public:
    RustSliderStyle(SharedFn track, SharedFn fill, SharedFn thumb)
        : track_(std::move(track)), fill_(std::move(fill)), thumb_(std::move(thumb)) {}
    View make_track(const SliderConfiguration& slider) const override { return call(*track_, slider); }
    View make_fill(const SliderConfiguration& slider) const override {
        return fill_ ? call(*fill_, slider) : SliderStyle::make_fill(slider);
    }
    View make_thumb(const SliderConfiguration& slider) const override { return call(*thumb_, slider); }

private:
    SharedFn track_, fill_, thumb_;
};

class RustMenuStyle final : public MenuStyle {
public:
    RustMenuStyle(SharedFn body, SharedFn item, std::optional<PopupMotion> motion)
        : body_(std::move(body)), item_(std::move(item)), motion_(std::move(motion)) {}
    View make_body(const MenuConfiguration& menu) const override { return call(*body_, menu); }
    View make_item(const MenuItemConfiguration& item) const override {
        return item_ ? call(*item_, item) : MenuStyle::make_item(item);
    }
    PopupMotion motion() const override { return motion_ ? *motion_ : MenuStyle::motion(); }

private:
    SharedFn body_, item_;
    std::optional<PopupMotion> motion_;
};

template <class Style, class Concrete, class... Args>
std::shared_ptr<const Style>* make_style(Args&&... args) {
    return new std::shared_ptr<const Style>(std::make_shared<const Concrete>(std::forward<Args>(args)...));
}

SharedFn owned(ElgClosure closure) {
    return std::make_shared<const RustFn>(closure);
}

std::optional<Prop<Color>> optional_color(const ElgProp* p) {
    return p ? std::optional<Prop<Color>>(prop<Color>(*p)) : std::nullopt;
}

} // namespace

// Per style family: clone and free the handle, set it for a subtree, and copy a configuration.
#define STYLE_HANDLE(name, Style)                                                                                    \
    ELG_EXPORT std::shared_ptr<const Style>* elg_##name##_style_clone(const std::shared_ptr<const Style>* s) {       \
        return new std::shared_ptr<const Style>(*s);                                                                 \
    }                                                                                                                \
    ELG_EXPORT void elg_##name##_style_free(std::shared_ptr<const Style>* s) {                                       \
        delete s;                                                                                                    \
    }                                                                                                                \
    ELG_EXPORT View* elg_mod_##name##_style(const View* v, const std::shared_ptr<const Style>* style) {             \
        ELG_TRY return out(v->modifier(Style##Modifier(*style)));                                                    \
        ELG_CATCH(nullptr)                                                                                           \
    }

#define CONFIG_HANDLE(name, Config)                                                                                  \
    ELG_EXPORT Config* elg_##name##_config_clone(const Config* c) {                                                  \
        return new Config(*c);                                                                                       \
    }                                                                                                                \
    ELG_EXPORT void elg_##name##_config_free(Config* c) {                                                            \
        delete c;                                                                                                    \
    }

// The style's body for a configuration (to wrap or reuse the current style).
#define MAKE_BODY(name, Style, Config)                                                                               \
    ELG_EXPORT View* elg_##name##_style_make_body(const std::shared_ptr<const Style>* s, const Config* c) {         \
        ELG_TRY return out((*s)->make_body(*c));                                                                     \
        ELG_CATCH(nullptr)                                                                                           \
    }

// A Rust style with make_body only.
#define RUST_STYLE(name, Style, Config)                                                                              \
    ELG_EXPORT std::shared_ptr<const Style>* elg_##name##_style_rust(ElgClosure make_body) {                         \
        ELG_TRY return make_style<Style, RustStyle<Style, Config>>(owned(make_body));                                \
        ELG_CATCH(nullptr)                                                                                           \
    }

#define STATE(which, member)                                                                                         \
    case which: return c->member()

// --- Buttons ---

STYLE_HANDLE(button, ButtonStyle)
CONFIG_HANDLE(button, ButtonConfiguration)
MAKE_BODY(button, ButtonStyle, ButtonConfiguration)
RUST_STYLE(button, ButtonStyle, ButtonConfiguration)

// kind: 0 prominent, 1 bordered, 2 plain.
ELG_EXPORT std::shared_ptr<const ButtonStyle>* elg_button_style_builtin(int32_t kind) {
    switch (kind) {
    case 0: return make_style<ButtonStyle, ProminentButtonStyle>();
    case 1: return make_style<ButtonStyle, BorderedButtonStyle>();
    default: return make_style<ButtonStyle, PlainButtonStyle>();
    }
}
ELG_EXPORT View* elg_button_config_label(const ButtonConfiguration* c) {
    return new View(c->label);
}
// which: 0 pressed, 1 hovered, 2 enabled, 3 focused.
ELG_EXPORT bool elg_button_config_state(const ButtonConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, is_pressed);
        STATE(1, is_hovered);
        STATE(2, is_enabled);
    default: return c->is_focused();
    }
    ELG_CATCH(false)
}

// --- Toggles ---

STYLE_HANDLE(toggle, ToggleStyle)
CONFIG_HANDLE(toggle, ToggleConfiguration)
MAKE_BODY(toggle, ToggleStyle, ToggleConfiguration)

// animation null: flips at once.
ELG_EXPORT std::shared_ptr<const ToggleStyle>* elg_toggle_style_rust(ElgClosure make_body, const Animation* animation) {
    ELG_TRY
    return make_style<ToggleStyle, RustAnimatedStyle<ToggleStyle, ToggleConfiguration>>(owned(make_body),
                                                                                          elg_shim::animation(animation));
    ELG_CATCH(nullptr)
}

struct ElgSwitchToggleStyleOptions {
    const ElgProp* on; // null: the tint
    ElgProp off;
    ElgProp knob;
    float width, height, knob_inset, spacing;
    bool fills_width;
    const Animation* animation; // null: flips at once
};

ELG_EXPORT std::shared_ptr<const ToggleStyle>* elg_switch_toggle_style(const ElgSwitchToggleStyleOptions* o) {
    ELG_TRY
    SwitchToggleStyleOptions options;
    options.on = optional_color(o->on);
    options.off = prop<Color>(o->off);
    options.knob = prop<Color>(o->knob);
    options.width = o->width;
    options.height = o->height;
    options.knob_inset = o->knob_inset;
    options.spacing = o->spacing;
    options.fills_width = o->fills_width;
    options.animation = animation(o->animation);
    return make_style<ToggleStyle, SwitchToggleStyle>(std::move(options));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_toggle_config_label(const ToggleConfiguration* c) {
    return new View(c->label);
}
// which: 0 on, 1 pressed, 2 hovered, 3 enabled, 4 focused.
ELG_EXPORT bool elg_toggle_config_state(const ToggleConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, is_on);
        STATE(1, is_pressed);
        STATE(2, is_hovered);
        STATE(3, is_enabled);
    default: return c->is_focused();
    }
    ELG_CATCH(false)
}
ELG_EXPORT void elg_toggle_config_toggle(const ToggleConfiguration* c) {
    ELG_TRY
    c->toggle();
    ELG_CATCH_VOID
}

// --- Check boxes ---

STYLE_HANDLE(checkbox, CheckboxStyle)
CONFIG_HANDLE(checkbox, CheckboxConfiguration)
MAKE_BODY(checkbox, CheckboxStyle, CheckboxConfiguration)

ELG_EXPORT std::shared_ptr<const CheckboxStyle>* elg_checkbox_style_rust(ElgClosure make_body, const Animation* animation) {
    ELG_TRY
    return make_style<CheckboxStyle, RustAnimatedStyle<CheckboxStyle, CheckboxConfiguration>>(owned(make_body),
                                                                                              elg_shim::animation(animation));
    ELG_CATCH(nullptr)
}

struct ElgBoxCheckboxStyleOptions {
    float size, corner_radius;
    const ElgProp* fill; // null: the tint
    ElgProp empty;
    ElgProp border;
    float border_width;
    ElgProp mark;
    View* take_check; // null: a check mark
    View* take_dash;  // null: a dash
    float spacing;
    const Animation* animation;
};

ELG_EXPORT std::shared_ptr<const CheckboxStyle>* elg_box_checkbox_style(const ElgBoxCheckboxStyleOptions* o) {
    ELG_TRY
    BoxCheckboxStyleOptions options;
    options.size = o->size;
    options.corner_radius = o->corner_radius;
    options.fill = optional_color(o->fill);
    options.empty = prop<Color>(o->empty);
    options.border = prop<Color>(o->border);
    options.border_width = o->border_width;
    options.mark = prop<Color>(o->mark);
    if (o->take_check) {
        options.check = take(o->take_check);
    }
    if (o->take_dash) {
        options.dash = take(o->take_dash);
    }
    options.spacing = o->spacing;
    options.animation = animation(o->animation);
    return make_style<CheckboxStyle, BoxCheckboxStyle>(std::move(options));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_checkbox_config_label(const CheckboxConfiguration* c) {
    return new View(c->label);
}
// which: 0 checked, 1 mixed, 2 pressed, 3 hovered, 4 enabled, 5 focused.
ELG_EXPORT bool elg_checkbox_config_state(const CheckboxConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, is_checked);
        STATE(1, is_mixed);
        STATE(2, is_pressed);
        STATE(3, is_hovered);
        STATE(4, is_enabled);
    default: return c->is_focused();
    }
    ELG_CATCH(false)
}
ELG_EXPORT void elg_checkbox_config_toggle(const CheckboxConfiguration* c) {
    ELG_TRY
    c->toggle();
    ELG_CATCH_VOID
}

// --- Number inputs ---

STYLE_HANDLE(number_input, NumberInputStyle)
CONFIG_HANDLE(number_input, NumberInputConfiguration)
MAKE_BODY(number_input, NumberInputStyle, NumberInputConfiguration)
RUST_STYLE(number_input, NumberInputStyle, NumberInputConfiguration)

struct ElgBoxNumberInputStyleOptions {
    ElgProp background;
    ElgProp border;
    const ElgProp* focused_border; // null: the tint
    float border_width, corner_radius;
    ElgEdgeInsets padding;
    float arrow_size;
};

// o null: PlainNumberInputStyle.
ELG_EXPORT std::shared_ptr<const NumberInputStyle>* elg_number_input_style_builtin(const ElgBoxNumberInputStyleOptions* o) {
    ELG_TRY
    if (!o) {
        return make_style<NumberInputStyle, PlainNumberInputStyle>();
    }
    BoxNumberInputStyleOptions options;
    options.background = prop<Color>(o->background);
    options.border = prop<Color>(o->border);
    options.focused_border = optional_color(o->focused_border);
    options.border_width = o->border_width;
    options.corner_radius = o->corner_radius;
    options.padding = edge_insets(o->padding);
    options.arrow_size = o->arrow_size;
    return make_style<NumberInputStyle, BoxNumberInputStyle>(std::move(options));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_number_input_config_field(const NumberInputConfiguration* c) {
    return new View(c->field);
}
// which: 0 can_increment, 1 can_decrement, 2 shows_buttons, 3 focused, 4 hovered, 5 enabled.
ELG_EXPORT bool elg_number_input_config_state(const NumberInputConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, can_increment);
        STATE(1, can_decrement);
        STATE(2, shows_buttons);
        STATE(3, is_focused);
        STATE(4, is_hovered);
    default: return c->is_enabled();
    }
    ELG_CATCH(false)
}
// which: 0 increment, 1 decrement.
ELG_EXPORT void elg_number_input_config_action(const NumberInputConfiguration* c, int32_t which) {
    ELG_TRY
    if (which == 0) {
        c->increment();
    } else {
        c->decrement();
    }
    ELG_CATCH_VOID
}

// --- Progress ---

STYLE_HANDLE(progress, ProgressStyle)
CONFIG_HANDLE(progress, ProgressConfiguration)
MAKE_BODY(progress, ProgressStyle, ProgressConfiguration)
RUST_STYLE(progress, ProgressStyle, ProgressConfiguration)

struct ElgProgressStyleOptions {
    ElgProp track;
    const ElgProp* fill; // null: the tint
    float a;             // linear: thickness; circular: size
    float b;             // circular: line_width
};

// kind: 0 LinearProgressStyle, 1 CircularProgressStyle.
ELG_EXPORT std::shared_ptr<const ProgressStyle>* elg_progress_style_builtin(int32_t kind, const ElgProgressStyleOptions* o) {
    ELG_TRY
    if (kind == 0) {
        return make_style<ProgressStyle, LinearProgressStyle>(
            LinearProgressStyleOptions{prop<Color>(o->track), optional_color(o->fill), o->a});
    }
    return make_style<ProgressStyle, CircularProgressStyle>(
        CircularProgressStyleOptions{prop<Color>(o->track), optional_color(o->fill), o->a, o->b});
    ELG_CATCH(nullptr)
}
// A tracked read; has false: indeterminate.
ELG_EXPORT ElgOptLen elg_progress_config_fraction(const ProgressConfiguration* c) {
    ELG_TRY
    const std::optional<float> fraction = c->fraction();
    return {fraction.has_value(), fraction.value_or(0)};
    ELG_CATCH(ElgOptLen{})
}

// --- Text fields ---

STYLE_HANDLE(text_field, TextFieldStyle)
CONFIG_HANDLE(text_field, TextFieldConfiguration)
MAKE_BODY(text_field, TextFieldStyle, TextFieldConfiguration)
RUST_STYLE(text_field, TextFieldStyle, TextFieldConfiguration)

// kind: 0 rounded border, 1 plain.
ELG_EXPORT std::shared_ptr<const TextFieldStyle>* elg_text_field_style_builtin(int32_t kind) {
    if (kind == 0) {
        return make_style<TextFieldStyle, RoundedBorderTextFieldStyle>();
    }
    return make_style<TextFieldStyle, PlainTextFieldStyle>();
}
ELG_EXPORT View* elg_text_field_config_field(const TextFieldConfiguration* c) {
    return new View(c->field);
}
// which: 0 focused, 1 hovered, 2 enabled.
ELG_EXPORT bool elg_text_field_config_state(const TextFieldConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, is_focused);
        STATE(1, is_hovered);
    default: return c->is_enabled();
    }
    ELG_CATCH(false)
}

// --- Pickers ---

STYLE_HANDLE(picker, PickerStyle)
CONFIG_HANDLE(picker, PickerConfiguration)
MAKE_BODY(picker, PickerStyle, PickerConfiguration)
RUST_STYLE(picker, PickerStyle, PickerConfiguration)

// kind: 0 segmented, 1 menu.
ELG_EXPORT std::shared_ptr<const PickerStyle>* elg_picker_style_builtin(int32_t kind) {
    if (kind == 0) {
        return make_style<PickerStyle, SegmentedPickerStyle>();
    }
    return make_style<PickerStyle, MenuPickerStyle>();
}
ELG_EXPORT size_t elg_picker_config_count(const PickerConfiguration* c) {
    return c->labels.size();
}
// Borrowed from the configuration.
ELG_EXPORT ElgStr elg_picker_config_label(const PickerConfiguration* c, size_t index) {
    return c_str(c->labels.at(index));
}
// A tracked read; the count while none is chosen.
ELG_EXPORT size_t elg_picker_config_selected(const PickerConfiguration* c) {
    ELG_TRY
    return c->selected();
    ELG_CATCH(c->labels.size())
}
ELG_EXPORT void elg_picker_config_select(const PickerConfiguration* c, size_t index) {
    ELG_TRY
    c->select(index);
    ELG_CATCH_VOID
}

// --- Sliders ---

STYLE_HANDLE(slider, SliderStyle)
CONFIG_HANDLE(slider, SliderConfiguration)

// fill may be none (nothing over the track).
ELG_EXPORT std::shared_ptr<const SliderStyle>* elg_slider_style_rust(ElgClosure track, ElgClosure fill, ElgClosure thumb) {
    ELG_TRY
    return make_style<SliderStyle, RustSliderStyle>(owned(track), share(fill), owned(thumb));
    ELG_CATCH(nullptr)
}

struct ElgKnobSliderStyleOptions {
    ElgProp track;
    const ElgProp* fill; // null: the tint
    float track_thickness;
    float knob_size;
};

ELG_EXPORT std::shared_ptr<const SliderStyle>* elg_knob_slider_style(const ElgKnobSliderStyleOptions* o) {
    ELG_TRY
    return make_style<SliderStyle, KnobSliderStyle>(
        KnobSliderStyleOptions{prop<Color>(o->track), optional_color(o->fill), o->track_thickness, o->knob_size});
    ELG_CATCH(nullptr)
}
// part: 0 track, 1 fill, 2 thumb.
ELG_EXPORT View* elg_slider_style_make_part(const std::shared_ptr<const SliderStyle>* s, const SliderConfiguration* c,
                                            int32_t part) {
    ELG_TRY
    switch (part) {
    case 0: return out((*s)->make_track(*c));
    case 1: return out((*s)->make_fill(*c));
    default: return out((*s)->make_thumb(*c));
    }
    ELG_CATCH(nullptr)
}
// A tracked read.
ELG_EXPORT float elg_slider_config_fraction(const SliderConfiguration* c) {
    ELG_TRY
    return c->fraction();
    ELG_CATCH(0.f)
}
// which: 0 pressed, 1 hovered, 2 enabled, 3 focused.
ELG_EXPORT bool elg_slider_config_state(const SliderConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, is_pressed);
        STATE(1, is_hovered);
        STATE(2, is_enabled);
    default: return c->is_focused();
    }
    ELG_CATCH(false)
}

// --- Navigation stacks ---

STYLE_HANDLE(navigation_stack, NavigationStackStyle)
CONFIG_HANDLE(navigation_stack, NavigationStackConfiguration)
MAKE_BODY(navigation_stack, NavigationStackStyle, NavigationStackConfiguration)
RUST_STYLE(navigation_stack, NavigationStackStyle, NavigationStackConfiguration)

struct ElgBarNavigationStackStyleOptions {
    float height;
    ElgProp background;
    ElgFont title_font;
    const ElgProp* back_color; // null: the tint
    bool shows_divider;
};

// o null: PlainNavigationStackStyle.
ELG_EXPORT std::shared_ptr<const NavigationStackStyle>* elg_navigation_stack_style_builtin(
    const ElgBarNavigationStackStyleOptions* o) {
    ELG_TRY
    if (!o) {
        return make_style<NavigationStackStyle, PlainNavigationStackStyle>();
    }
    BarNavigationStackStyleOptions options;
    options.height = o->height;
    options.background = prop<Color>(o->background);
    options.title_font = font(o->title_font);
    options.back_color = optional_color(o->back_color);
    options.shows_divider = o->shows_divider;
    return make_style<NavigationStackStyle, BarNavigationStackStyle>(std::move(options));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_navigation_stack_config_pages(const NavigationStackConfiguration* c) {
    return new View(c->pages);
}
// Calls `each` with each title, the root's first. A tracked read.
ELG_EXPORT void elg_navigation_stack_config_titles(const NavigationStackConfiguration* c, ElgStrSink each, void* ctx) {
    ELG_TRY
    for (const std::string& title : c->titles()) {
        sink(each, ctx, title);
    }
    ELG_CATCH_VOID
}
ELG_EXPORT bool elg_navigation_stack_config_can_go_back(const NavigationStackConfiguration* c) {
    ELG_TRY
    return c->can_go_back();
    ELG_CATCH(false)
}
ELG_EXPORT void elg_navigation_stack_config_back(const NavigationStackConfiguration* c) {
    ELG_TRY
    c->back();
    ELG_CATCH_VOID
}

// --- Popovers ---

STYLE_HANDLE(popover, PopoverStyle)
CONFIG_HANDLE(popover, PopoverConfiguration)
MAKE_BODY(popover, PopoverStyle, PopoverConfiguration)

ELG_EXPORT std::shared_ptr<const PopoverStyle>* elg_popover_style_rust(ElgClosure make_body, ElgPopupMotion popup_motion) {
    ELG_TRY
    return make_style<PopoverStyle, RustPopupStyle<PopoverStyle, PopoverConfiguration>>(owned(make_body), motion(popup_motion));
    ELG_CATCH(nullptr)
}
// kind: 0 default, 1 plain.
ELG_EXPORT std::shared_ptr<const PopoverStyle>* elg_popover_style_builtin(int32_t kind) {
    if (kind == 0) {
        return make_style<PopoverStyle, DefaultPopoverStyle>();
    }
    return make_style<PopoverStyle, PlainPopoverStyle>();
}
ELG_EXPORT View* elg_popover_config_content(const PopoverConfiguration* c) {
    return new View(c->content);
}
ELG_EXPORT bool elg_popover_config_is_padded(const PopoverConfiguration* c) {
    ELG_TRY
    return c->is_padded();
    ELG_CATCH(true)
}

// --- Tooltips ---

STYLE_HANDLE(tooltip, TooltipStyle)
CONFIG_HANDLE(tooltip, TooltipConfiguration)
MAKE_BODY(tooltip, TooltipStyle, TooltipConfiguration)

ELG_EXPORT std::shared_ptr<const TooltipStyle>* elg_tooltip_style_rust(ElgClosure make_body, ElgPopupMotion popup_motion) {
    ELG_TRY
    return make_style<TooltipStyle, RustPopupStyle<TooltipStyle, TooltipConfiguration>>(owned(make_body), motion(popup_motion));
    ELG_CATCH(nullptr)
}
// kind: 0 default, 1 plain.
ELG_EXPORT std::shared_ptr<const TooltipStyle>* elg_tooltip_style_builtin(int32_t kind) {
    if (kind == 0) {
        return make_style<TooltipStyle, DefaultTooltipStyle>();
    }
    return make_style<TooltipStyle, PlainTooltipStyle>();
}
ELG_EXPORT View* elg_tooltip_config_content(const TooltipConfiguration* c) {
    return new View(c->content);
}

// --- Menus ---

STYLE_HANDLE(menu, MenuStyle)
CONFIG_HANDLE(menu, MenuConfiguration)
CONFIG_HANDLE(menu_item, MenuItemConfiguration)
MAKE_BODY(menu, MenuStyle, MenuConfiguration)

// make_item (args: MenuItemConfiguration*, ret: View*) may be none: the default row.
ELG_EXPORT std::shared_ptr<const MenuStyle>* elg_menu_style_rust(ElgClosure make_body, ElgClosure make_item,
                                                                  ElgPopupMotion popup_motion) {
    ELG_TRY
    return make_style<MenuStyle, RustMenuStyle>(owned(make_body), share(make_item), motion(popup_motion));
    ELG_CATCH(nullptr)
}
// kind: 0 default, 1 plain.
ELG_EXPORT std::shared_ptr<const MenuStyle>* elg_menu_style_builtin(int32_t kind) {
    if (kind == 0) {
        return make_style<MenuStyle, DefaultMenuStyle>();
    }
    return make_style<MenuStyle, PlainMenuStyle>();
}
ELG_EXPORT View* elg_menu_style_make_item(const std::shared_ptr<const MenuStyle>* s, const MenuItemConfiguration* c) {
    ELG_TRY
    return out((*s)->make_item(*c));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_menu_config_items(const MenuConfiguration* c) {
    return new View(c->items);
}
// which: 0 title, 1 leading, 2 trailing.
ELG_EXPORT View* elg_menu_item_config_view(const MenuItemConfiguration* c, int32_t which) {
    switch (which) {
    case 0: return new View(c->title);
    case 1: return new View(c->leading);
    default: return new View(c->trailing);
    }
}
// which: 0 highlighted, 1 enabled, 2 destructive.
ELG_EXPORT bool elg_menu_item_config_state(const MenuItemConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, is_highlighted);
        STATE(1, is_enabled);
    default: return c->is_destructive();
    }
    ELG_CATCH(false)
}

// --- Selectable rows ---

STYLE_HANDLE(row, RowStyle)
CONFIG_HANDLE(row, RowConfiguration)
MAKE_BODY(row, RowStyle, RowConfiguration)
RUST_STYLE(row, RowStyle, RowConfiguration)

struct ElgHighlightRowStyleOptions {
    const ElgProp* selected; // null: the tint
    ElgProp on_selected;
    ElgProp hovered;
    float corner_radius;
    ElgEdgeInsets padding;
};

// o null: PlainRowStyle.
ELG_EXPORT std::shared_ptr<const RowStyle>* elg_row_style_builtin(const ElgHighlightRowStyleOptions* o) {
    ELG_TRY
    if (!o) {
        return make_style<RowStyle, PlainRowStyle>();
    }
    return make_style<RowStyle, HighlightRowStyle>(HighlightRowStyleOptions{
        optional_color(o->selected), prop<Color>(o->on_selected), prop<Color>(o->hovered), o->corner_radius,
        edge_insets(o->padding)});
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_row_config_content(const RowConfiguration* c) {
    return new View(c->content);
}
ELG_EXPORT int32_t elg_row_config_axis(const RowConfiguration* c) {
    return static_cast<int32_t>(c->axis);
}
// which: 0 selected, 1 hovered, 2 focused, 3 enabled.
ELG_EXPORT bool elg_row_config_state(const RowConfiguration* c, int32_t which) {
    ELG_TRY
    switch (which) {
        STATE(0, is_selected);
        STATE(1, is_hovered);
        STATE(2, is_focused);
    default: return c->is_enabled();
    }
    ELG_CATCH(false)
}

// --- ABI layout ---

ELG_EXPORT void elg_layouts_styles(ElgStrSink out, void* ctx) {
    ELG_LAYOUT(ElgSwitchToggleStyleOptions);
    ELG_LAYOUT(ElgBoxCheckboxStyleOptions);
    ELG_LAYOUT(ElgBoxNumberInputStyleOptions);
    ELG_LAYOUT(ElgProgressStyleOptions);
    ELG_LAYOUT(ElgKnobSliderStyleOptions);
    ELG_LAYOUT(ElgBarNavigationStackStyleOptions);
    ELG_LAYOUT(ElgHighlightRowStyleOptions);
}
