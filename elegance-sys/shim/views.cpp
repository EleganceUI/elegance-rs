// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// https://mozilla.org/MPL/2.0/.

// Built-in views and modifiers, text, shapes, paths, drawing, images, menus, navigation.

#include "shim.h"

#include <array>
#include <mutex>

using namespace elg;
using namespace elg_shim;

#define VIEW_TRY(...)                 \
    ELG_TRY return out(__VA_ARGS__); \
    ELG_CATCH(nullptr)

// --- Containers ---

ELG_EXPORT View* elg_vstack(int32_t alignment, ElgOptLen spacing, View** children, size_t count) {
    VIEW_TRY(VStackLayout(static_cast<HorizontalAlignment>(alignment), length(spacing))(take_all(children, count)))
}

ELG_EXPORT View* elg_hstack(int32_t alignment, ElgOptLen spacing, View** children, size_t count) {
    VIEW_TRY(HStackLayout(static_cast<VerticalAlignment>(alignment), length(spacing))(take_all(children, count)))
}

ELG_EXPORT View* elg_zstack(ElgAlignment alignment, View** children, size_t count) {
    VIEW_TRY(ZStackLayout(elg_shim::alignment(alignment))(take_all(children, count)))
}

ELG_EXPORT View* elg_spacer(ElgOptLen min_length) {
    VIEW_TRY(Spacer(length(min_length)))
}

ELG_EXPORT View* elg_empty_view() {
    VIEW_TRY(EmptyView())
}

ELG_EXPORT ElgPoint elg_aligned_position(ElgRect container, ElgSize content, ElgAlignment a) {
    const Point p = aligned_position(rect(container), size(content), alignment(a));
    return {p.x, p.y};
}

// Stack layouts used from custom layouts.
ELG_EXPORT ElgSize elg_stack_layout_size(int32_t kind, ElgAlignment a, ElgOptLen spacing, ElgProposedSize proposal,
                                         const LayoutChild* children, size_t count) {
    ELG_TRY
    const std::span<const LayoutChild> span(children, count);
    Size result;
    if (kind == 0) {
        result = VStackLayout(static_cast<HorizontalAlignment>(a.horizontal), length(spacing)).size_that_fits(proposed(proposal), span);
    } else if (kind == 1) {
        result = HStackLayout(static_cast<VerticalAlignment>(a.vertical), length(spacing)).size_that_fits(proposed(proposal), span);
    } else {
        result = ZStackLayout(alignment(a)).size_that_fits(proposed(proposal), span);
    }
    return {result.width, result.height};
    ELG_CATCH(ElgSize{})
}

ELG_EXPORT void elg_stack_layout_place(int32_t kind, ElgAlignment a, ElgOptLen spacing, ElgRect bounds,
                                       ElgProposedSize proposal, const LayoutChild* children, size_t count) {
    ELG_TRY
    const std::span<const LayoutChild> span(children, count);
    if (kind == 0) {
        VStackLayout(static_cast<HorizontalAlignment>(a.horizontal), length(spacing)).place_children(rect(bounds), proposed(proposal), span);
    } else if (kind == 1) {
        HStackLayout(static_cast<VerticalAlignment>(a.vertical), length(spacing)).place_children(rect(bounds), proposed(proposal), span);
    } else {
        ZStackLayout(alignment(a)).place_children(rect(bounds), proposed(proposal), span);
    }
    ELG_CATCH_VOID
}

// --- Controls ---

ELG_EXPORT View* elg_button(View* take_label, ElgClosure on_click) {
    VIEW_TRY(Button(take(take_label), action(on_click)))
}

ELG_EXPORT View* elg_button_title(ElgProp title, ElgClosure on_click) {
    VIEW_TRY(Button(prop<std::string>(title), action(on_click)))
}

// For TextField and TextArea (which has no `secure` or `line_limit`).
struct ElgTextFieldOptions {
    ElgClosure on_submit;
    ElgProp secure;
    ElgProp line_limit;
    // args: ElgStr* text, ret: std::vector<TextSpan>* (filled with elg_text_spans_push).
    ElgClosure styler;
    // args: ElgKeyEvent*, ret: bool*.
    ElgClosure on_key;
};

static std::function<std::vector<TextSpan>(std::string_view)> text_styler(ElgClosure styler) {
    SharedFn f = share(styler);
    if (!f) {
        return {};
    }
    return [f](std::string_view text) {
        std::vector<TextSpan> spans;
        ElgStr arg{text.data(), text.size()};
        f->call(&arg, &spans);
        return spans;
    };
}

static std::function<bool(const KeyEvent&)> key_handler(ElgClosure on_key) {
    SharedFn f = share(on_key);
    if (!f) {
        return {};
    }
    return [f](const KeyEvent& event) {
        ElgKeyEvent arg = c_key_event(event);
        bool handled = false;
        f->call(&arg, &handled);
        return handled;
    };
}

ELG_EXPORT void elg_text_spans_push(std::vector<TextSpan>* spans, size_t begin, size_t end, const ElgRunStyle* style,
                                    const ElgStr* label) {
    ELG_TRY
    TextSpan span;
    span.begin = begin;
    span.end = end;
    span.style = run_style(*style);
    if (label) {
        span.label = str(*label);
    }
    spans->push_back(std::move(span));
    ELG_CATCH_VOID
}

ELG_EXPORT View* elg_text_field(ElgProp placeholder, Binding<std::string>* take_text, const ElgTextFieldOptions* o) {
    ELG_TRY
    std::unique_ptr<Binding<std::string>> text(take_text);
    TextFieldOptions options;
    options.on_submit = action(o->on_submit);
    options.secure = prop<bool>(o->secure);
    options.line_limit = prop<int>(o->line_limit);
    options.styler = text_styler(o->styler);
    options.on_key = key_handler(o->on_key);
    return out(TextField(prop<std::string>(placeholder), *text, std::move(options)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_text_area(ElgProp placeholder, Binding<std::string>* take_text, const ElgTextFieldOptions* o) {
    ELG_TRY
    std::unique_ptr<Binding<std::string>> text(take_text);
    TextAreaOptions options;
    options.on_submit = action(o->on_submit);
    options.styler = text_styler(o->styler);
    options.on_key = key_handler(o->on_key);
    return out(TextArea(prop<std::string>(placeholder), *text, std::move(options)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_slider(Binding<float>* take_value, ElgProp min, ElgProp max, ElgProp step, ElgClosure on_editing_changed) {
    ELG_TRY
    std::unique_ptr<Binding<float>> value(take_value);
    SliderOptions options;
    options.min = prop<float>(min);
    options.max = prop<float>(max);
    options.step = prop<float>(step);
    options.on_editing_changed = consumer<bool>(on_editing_changed);
    return out(Slider(*value, std::move(options)));
    ELG_CATCH(nullptr)
}

struct ElgNumberInputOptions {
    ElgProp min, max, step;
    ElgProp decimals;
    ElgClosure format; // args: double*, ret: std::string*
    ElgClosure parse;  // args: ElgStr*, ret: ElgOptNumber*
    ElgProp live;
    ElgProp shows_buttons;
    ElgProp scrubs;
};

struct ElgOptNumber {
    bool has;
    double value;
};

ELG_EXPORT View* elg_number_input(Binding<double>* take_value, bool integer, const ElgNumberInputOptions* o) {
    ELG_TRY
    std::unique_ptr<Binding<double>> value(take_value);
    NumberInputOptions options;
    options.min = prop<double>(o->min);
    options.max = prop<double>(o->max);
    options.step = prop<double>(o->step);
    options.decimals = prop<int>(o->decimals);
    if (SharedFn f = share(o->format)) {
        options.format = [f](double v) {
            std::string result;
            f->call(&v, &result);
            return result;
        };
    }
    if (SharedFn f = share(o->parse)) {
        options.parse = [f](std::string_view text) {
            ElgStr arg{text.data(), text.size()};
            ElgOptNumber result{};
            f->call(&arg, &result);
            return result.has ? std::optional<double>(result.value) : std::nullopt;
        };
    }
    options.live = prop<bool>(o->live);
    options.shows_buttons = prop<bool>(o->shows_buttons);
    options.scrubs = prop<bool>(o->scrubs);
    return out(detail::make_number_input(*value, integer, std::move(options)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_progress(ElgProp fraction) {
    VIEW_TRY(Progress(prop<std::optional<float>>(fraction)))
}

// `selected` (ret: size_t*) is the chosen option's index (count: none); `select` (args: size_t*).
ELG_EXPORT View* elg_picker(const ElgStr* labels, size_t count, ElgClosure selected, ElgClosure select) {
    ELG_TRY
    std::vector<std::string> names;
    names.reserve(count);
    for (size_t i = 0; i < count; ++i) {
        names.push_back(str(labels[i]));
    }
    return out(detail::make_picker(std::move(names), producer<std::size_t>(selected), consumer<std::size_t>(select)));
    ELG_CATCH(nullptr)
}

// take_label null: no label.
ELG_EXPORT View* elg_toggle(View* take_label, Binding<bool>* take_is_on) {
    ELG_TRY
    std::unique_ptr<Binding<bool>> is_on(take_is_on);
    if (!take_label) {
        return out(Toggle(*is_on));
    }
    return out(Toggle(take(take_label), *is_on));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_toggle_title(ElgProp title, Binding<bool>* take_is_on) {
    ELG_TRY
    std::unique_ptr<Binding<bool>> is_on(take_is_on);
    return out(Toggle(prop<std::string>(title), *is_on));
    ELG_CATCH(nullptr)
}

// take_label null: no label.
ELG_EXPORT View* elg_checkbox(View* take_label, Binding<bool>* take_is_checked, ElgProp mixed) {
    ELG_TRY
    std::unique_ptr<Binding<bool>> is_checked(take_is_checked);
    CheckboxOptions options{prop<bool>(mixed)};
    if (!take_label) {
        return out(Checkbox(*is_checked, std::move(options)));
    }
    return out(Checkbox(take(take_label), *is_checked, std::move(options)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_checkbox_title(ElgProp title, Binding<bool>* take_is_checked, ElgProp mixed) {
    ELG_TRY
    std::unique_ptr<Binding<bool>> is_checked(take_is_checked);
    return out(Checkbox(prop<std::string>(title), *is_checked, CheckboxOptions{prop<bool>(mixed)}));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_separator(int32_t axis, ElgProp color, float thickness, float inset) {
    VIEW_TRY(Separator(static_cast<Axis>(axis), {prop<Color>(color), thickness, inset}))
}

// --- Structure ---

ELG_EXPORT View* elg_if(ElgProp condition, View* take_then, View* take_else) {
    VIEW_TRY(If(prop<bool>(condition), take(take_then), take(take_else)))
}

// One ForEach item: `value` owns the Rust item; `equals` compares two items' Rust data.
struct ElgForEachItem {
    bool key_is_string;
    int64_t int_key;
    ElgStr string_key;
    ElgClosure value; // call: unused
    bool (*equals)(void* a, void* b);
    ElgClosure view; // ret: View*
};

namespace {

struct RustItem {
    std::shared_ptr<const RustFn> value;
    bool (*equals)(void*, void*);
};

bool rust_items_equal(const void* a, const void* b) {
    const auto* x = static_cast<const RustItem*>(a);
    const auto* y = static_cast<const RustItem*>(b);
    return x->equals == y->equals && x->equals(x->value->data(), y->value->data());
}

std::vector<detail::ForEachItem> items_from(const ElgForEachItem* items, size_t count) {
    std::vector<detail::ForEachItem> result;
    result.reserve(count);
    for (size_t i = 0; i < count; ++i) {
        const ElgForEachItem& item = items[i];
        ItemKey key = item.key_is_string ? ItemKey(str(item.string_key)) : ItemKey(item.int_key);
        auto value = std::make_shared<const RustItem>(RustItem{std::make_shared<const RustFn>(item.value), item.equals});
        result.push_back({std::move(key), value, &rust_items_equal, view_builder(item.view)});
    }
    return result;
}

} // namespace

// `items` (ret: std::vector<ForEachItem>*, filled with elg_for_each_items_push) is a tracked read.
ELG_EXPORT View* elg_for_each(ElgClosure items) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(items);
    return out(detail::make_for_each([f] {
        std::vector<detail::ForEachItem> result;
        f->call(nullptr, &result);
        return result;
    }));
    ELG_CATCH(nullptr)
}

ELG_EXPORT void elg_for_each_items_push(std::vector<detail::ForEachItem>* target, const ElgForEachItem* items, size_t count) {
    ELG_TRY
    for (auto& item : items_from(items, count)) {
        target->push_back(std::move(item));
    }
    ELG_CATCH_VOID
}

// --- Scrolling ---

ELG_EXPORT ScrollPosition* elg_scroll_position_new() {
    ELG_TRY
    return new ScrollPosition();
    ELG_CATCH(nullptr)
}
ELG_EXPORT ScrollPosition* elg_scroll_position_clone(const ScrollPosition* position) {
    return new ScrollPosition(*position);
}
ELG_EXPORT void elg_scroll_position_free(ScrollPosition* position) {
    delete position;
}
ELG_EXPORT void elg_scroll_position_to_top(const ScrollPosition* position, bool animated) {
    ELG_TRY
    position->scroll_to_top(animated);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_scroll_position_to_end(const ScrollPosition* position, bool animated) {
    ELG_TRY
    position->scroll_to_end(animated);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_scroll_position_to_int(const ScrollPosition* position, int64_t key) {
    ELG_TRY
    position->scroll_to(key);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_scroll_position_to_string(const ScrollPosition* position, ElgStr key) {
    ELG_TRY
    position->scroll_to(str(key));
    ELG_CATCH_VOID
}

ELG_EXPORT View* elg_scroll_view(View* take_content, int32_t axes, ElgProp shows_indicators, const ScrollPosition* position,
                                 ElgProp sticks_to_end) {
    ELG_TRY
    ScrollViewOptions options;
    options.axes = static_cast<ScrollAxes>(axes);
    options.shows_indicators = prop<bool>(shows_indicators);
    if (position) {
        options.position = *position;
    }
    options.sticks_to_end = prop<bool>(sticks_to_end);
    return out(ScrollView(std::move(options), take(take_content)));
    ELG_CATCH(nullptr)
}

struct ElgStackSeparators {
    ElgProp color;
    float inset;
    float thickness;
};

static std::optional<StackSeparators> stack_separators(const ElgStackSeparators* s) {
    if (!s) {
        return std::nullopt;
    }
    return StackSeparators{prop<Color>(s->color), s->inset, s->thickness};
}

// separators null: none.
ELG_EXPORT View* elg_lazy_stack(bool vertical, int32_t alignment, ElgOptLen spacing, const ElgStackSeparators* separators,
                                View* take_for_each) {
    ELG_TRY
    if (vertical) {
        return out(LazyVStack({static_cast<HorizontalAlignment>(alignment), length(spacing), stack_separators(separators)},
                              take(take_for_each)));
    }
    return out(LazyHStack({static_cast<VerticalAlignment>(alignment), length(spacing), stack_separators(separators)},
                          take(take_for_each)));
    ELG_CATCH(nullptr)
}

// --- Menus ---

ELG_EXPORT View* elg_menu_item(ElgProp title, ElgClosure on_choose, ElgProp shortcut, ElgProp destructive,
                               const ElgProp* checked, const ElgImageSource* image) {
    ELG_TRY
    MenuItemOptions options;
    options.shortcut = prop<std::string>(shortcut);
    options.destructive = prop<bool>(destructive);
    if (checked) {
        options.checked = prop<bool>(*checked);
    }
    if (image) {
        options.image = image_source(*image);
    }
    return out(MenuItem(prop<std::string>(title), action(on_choose), std::move(options)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_submenu(ElgProp title, ElgClosure menu) {
    VIEW_TRY(Submenu(prop<std::string>(title), view_builder(menu)))
}

ELG_EXPORT View* elg_menu(View** items, size_t count) {
    VIEW_TRY(detail::make_menu(take_all(items, count)))
}

// One of take_label and title: a View label, else the title.
ELG_EXPORT View* elg_menu_button(View* take_label, const ElgProp* title, ElgClosure menu, const ElgFloatingPlacement* placement,
                                 ElgProp shows_indicator, ElgPopupMotion popup_motion) {
    ELG_TRY
    MenuButtonOptions options;
    options.placement = elg_shim::placement(*placement);
    options.shows_indicator = prop<bool>(shows_indicator);
    options.motion = motion(popup_motion);
    if (take_label) {
        return out(MenuButton(take(take_label), view_builder(menu), std::move(options)));
    }
    return out(MenuButton(prop<std::string>(*title), view_builder(menu), std::move(options)));
    ELG_CATCH(nullptr)
}

// --- Navigation ---

struct ElgSplitViewOptions {
    ElgProp divider;
    const ElgProp* active_divider; // null: the tint
    float thickness;
    float active_thickness;
};

static SplitViewOptions split_view_options(const ElgSplitViewOptions& o) {
    SplitViewOptions result;
    result.divider = prop<Color>(o.divider);
    if (o.active_divider) {
        result.active_divider = prop<Color>(*o.active_divider);
    }
    result.thickness = o.thickness;
    result.active_thickness = o.active_thickness;
    return result;
}

ELG_EXPORT View* elg_split_view(int32_t axis, View** panes, size_t count, const ElgSplitViewOptions* options) {
    VIEW_TRY(detail::make_split_view(static_cast<Axis>(axis), take_all(panes, count), split_view_options(*options)))
}

struct ElgColumnWidth {
    float min, ideal, max;
};

struct ElgNavigationSplitViewOptions {
    Binding<bool>* take_sidebar_visible; // null: always shown
    const Transition* sidebar_transition;
    ElgProp sidebar_background;
    ElgColumnWidth sidebar_width;
    ElgColumnWidth content_width;
    ElgColumnWidth detail_width;
    ElgSplitViewOptions dividers;
};

ELG_EXPORT View* elg_navigation_split_view(View* take_sidebar, View* take_content, View* take_detail,
                                           const ElgNavigationSplitViewOptions* o) {
    ELG_TRY
    NavigationSplitViewOptions options;
    if (o->take_sidebar_visible) {
        std::unique_ptr<Binding<bool>> visible(o->take_sidebar_visible);
        options.sidebar_visible = *visible;
    }
    options.sidebar_transition = *o->sidebar_transition;
    options.sidebar_background = prop<Color>(o->sidebar_background);
    const auto width = [](ElgColumnWidth w) { return ColumnWidth{w.min, w.ideal, w.max}; };
    options.sidebar_width = width(o->sidebar_width);
    options.content_width = width(o->content_width);
    options.detail_width = width(o->detail_width);
    options.dividers = split_view_options(o->dividers);
    if (take_content) {
        return out(NavigationSplitView(take(take_sidebar), take(take_content), take(take_detail), std::move(options)));
    }
    return out(NavigationSplitView(take(take_sidebar), take(take_detail), std::move(options)));
    ELG_CATCH(nullptr)
}

// pages (ret: std::vector<ForEachItem>*), titles (ret: std::vector<std::string>*, filled with
// elg_strings_push), pop: tracked reads of the Rust path, as NavigationStack does.
// transition: PageTransition; animation null: pushes and pops not animated themselves change at once.
ELG_EXPORT View* elg_navigation_stack(View* take_root, ElgClosure pages, ElgClosure titles, ElgClosure pop,
                                      int32_t transition, const Animation* animation, bool back_gestures) {
    ELG_TRY
    SharedFn p = std::make_shared<const RustFn>(pages);
    SharedFn t = std::make_shared<const RustFn>(titles);
    return out(detail::make_navigation_stack(
        take(take_root),
        [p] {
            std::vector<detail::ForEachItem> result;
            p->call(nullptr, &result);
            return result;
        },
        [t] {
            std::vector<std::string> result;
            t->call(nullptr, &result);
            return result;
        },
        action(pop), {static_cast<PageTransition>(transition), elg_shim::animation(animation), back_gestures}));
    ELG_CATCH(nullptr)
}

ELG_EXPORT void elg_strings_push(std::vector<std::string>* target, ElgStr value) {
    target->push_back(str(value));
}

// --- Text ---

ELG_EXPORT Text* elg_text(ElgProp content) {
    ELG_TRY
    return new Text(prop<std::string>(content));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Text* elg_text_clone(const Text* text) {
    return new Text(*text);
}
ELG_EXPORT void elg_text_free(Text* text) {
    delete text;
}
ELG_EXPORT View* elg_text_to_view(const Text* text) {
    return new View(*text);
}
ELG_EXPORT Text* elg_text_font(const Text* text, ElgProp f) {
    ELG_TRY
    return new Text(text->font(prop<Font>(f)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Text* elg_text_font_weight(const Text* text, ElgProp weight) {
    ELG_TRY
    return new Text(text->font_weight(prop<Weight>(weight)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Text* elg_text_bold(const Text* text) {
    ELG_TRY
    return new Text(text->bold());
    ELG_CATCH(nullptr)
}
ELG_EXPORT Text* elg_text_foreground_style(const Text* text, ElgProp color) {
    ELG_TRY
    return new Text(text->foreground_style(prop<Color>(color)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Text* elg_text_line_limit(const Text* text, int32_t lines) {
    ELG_TRY
    return new Text(text->line_limit(lines));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Text* elg_text_multiline_alignment(const Text* text, int32_t alignment) {
    ELG_TRY
    return new Text(text->multiline_text_alignment(static_cast<TextAlignment>(alignment)));
    ELG_CATCH(nullptr)
}

struct ElgTextRun {
    ElgStr text;
    ElgRunStyle style;
    ElgClosure on_tap;
    View* take_attachment; // null: none
    float attachment_size;
};

ELG_EXPORT View* elg_styled_text(const ElgTextRun* runs, size_t count) {
    ELG_TRY
    std::vector<TextRun> result;
    result.reserve(count);
    for (size_t i = 0; i < count; ++i) {
        TextRun run;
        run.text = str(runs[i].text);
        run.style = run_style(runs[i].style);
        run.on_tap = action(runs[i].on_tap);
        if (runs[i].take_attachment) {
            run.attachment = take(runs[i].take_attachment);
        }
        run.attachment_size = runs[i].attachment_size;
        result.push_back(std::move(run));
    }
    return out(StyledText(std::move(result)));
    ELG_CATCH(nullptr)
}

// --- Shapes ---

ELG_EXPORT Shape* elg_shape_clone(const Shape* shape) {
    return new Shape(*shape);
}
ELG_EXPORT void elg_shape_free(Shape* shape) {
    delete shape;
}
ELG_EXPORT View* elg_shape_to_view(const Shape* shape) {
    return new View(*shape);
}
ELG_EXPORT Shape* elg_rectangle() {
    return new Shape(Rectangle());
}
ELG_EXPORT Shape* elg_rounded_rectangle(float top_left, float top_right, float bottom_right, float bottom_left) {
    ELG_TRY
    return new Shape(RoundedRectangle(CornerRadii{top_left, top_right, bottom_right, bottom_left}));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Shape* elg_capsule() {
    return new Shape(Capsule());
}
ELG_EXPORT Shape* elg_circle() {
    return new Shape(Circle());
}
ELG_EXPORT Shape* elg_path_shape(const Path* path, const ElgRect* view_box, int32_t fill_rule) {
    ELG_TRY
    PathShapeOptions options;
    if (view_box) {
        options.view_box = rect(*view_box);
    }
    options.fill_rule = static_cast<FillRule>(fill_rule);
    return new Shape(PathShape(*path, options));
    ELG_CATCH(nullptr)
}
// `path` (args: ElgRect*, ret: Path*) draws the outline for the given bounds.
ELG_EXPORT Shape* elg_custom_shape(ElgClosure path, int32_t fill_rule) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(path);
    return new Shape(CustomShape(
        [f](Rect bounds) {
            ElgRect arg = c_rect(bounds);
            Path result;
            f->call(&arg, &result);
            return result;
        },
        static_cast<FillRule>(fill_rule)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Shape* elg_shape_fill(const Shape* shape, ElgProp style) {
    ELG_TRY
    return new Shape(shape->fill(prop<ShapeStyle>(style)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Shape* elg_shape_stroke(const Shape* shape, ElgProp color, const ElgStrokeStyle* style) {
    ELG_TRY
    return new Shape(shape->stroke(prop<Color>(color), stroke_style(*style)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT Shape* elg_shape_stroke_border(const Shape* shape, ElgProp color, float line_width) {
    ELG_TRY
    return new Shape(shape->stroke_border(prop<Color>(color), line_width));
    ELG_CATCH(nullptr)
}

// --- Paths ---

ELG_EXPORT Path* elg_path_new() {
    return new Path();
}
ELG_EXPORT Path* elg_path_clone(const Path* path) {
    return new Path(*path);
}
ELG_EXPORT void elg_path_free(Path* path) {
    delete path;
}
ELG_EXPORT void elg_path_assign(Path* target, const Path* source) {
    *target = *source;
}
ELG_EXPORT bool elg_path_eq(const Path* a, const Path* b) {
    return *a == *b;
}
ELG_EXPORT void elg_path_move_to(Path* path, ElgPoint p) {
    path->move_to(point(p));
}
ELG_EXPORT void elg_path_line_to(Path* path, ElgPoint p) {
    path->line_to(point(p));
}
ELG_EXPORT void elg_path_quad_to(Path* path, ElgPoint control, ElgPoint end) {
    path->quad_to(point(control), point(end));
}
ELG_EXPORT void elg_path_cubic_to(Path* path, ElgPoint control1, ElgPoint control2, ElgPoint end) {
    path->cubic_to(point(control1), point(control2), point(end));
}
ELG_EXPORT void elg_path_arc_to(Path* path, float rx, float ry, float rotation, bool large_arc, bool sweep, ElgPoint end) {
    path->arc_to(rx, ry, rotation, large_arc, sweep, point(end));
}
ELG_EXPORT void elg_path_close(Path* path) {
    path->close();
}
ELG_EXPORT void elg_path_add_rect(Path* path, ElgRect r) {
    path->add_rect(rect(r));
}
ELG_EXPORT void elg_path_add_rounded_rect(Path* path, ElgRect r, float radius) {
    path->add_rounded_rect(rect(r), radius);
}
ELG_EXPORT void elg_path_add_ellipse(Path* path, ElgRect r) {
    path->add_ellipse(rect(r));
}
ELG_EXPORT void elg_path_add_circle(Path* path, ElgPoint center, float radius) {
    path->add_circle(point(center), radius);
}
ELG_EXPORT void elg_path_add_path(Path* path, const Path* other) {
    path->add_path(*other);
}
ELG_EXPORT Path* elg_path_svg(ElgStr data) {
    ELG_TRY
    return new Path(Path::svg(view(data)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT bool elg_path_empty(const Path* path) {
    return path->empty();
}
ELG_EXPORT ElgRect elg_path_bounds(const Path* path) {
    return c_rect(path->bounds());
}
ELG_EXPORT bool elg_path_contains(const Path* path, ElgPoint p, int32_t rule) {
    return path->contains(point(p), static_cast<FillRule>(rule));
}
ELG_EXPORT Path* elg_path_transformed(const Path* path, float sx, float sy, float dx, float dy) {
    return new Path(path->transformed(sx, sy, dx, dy));
}
// Calls `polyline` (args: an ElgPolyline*) for each flattened subpath.
struct ElgPolyline {
    const ElgPoint* points;
    size_t count;
    bool closed;
};
ELG_EXPORT void elg_path_flatten(const Path* path, float tolerance, ElgClosure polyline) {
    ELG_TRY
    for (const Path::Polyline& line : path->flatten(tolerance)) {
        static_assert(sizeof(Point) == sizeof(ElgPoint));
        ElgPolyline arg{reinterpret_cast<const ElgPoint*>(line.points.data()), line.points.size(), line.closed};
        call_now(polyline, &arg, nullptr);
    }
    ELG_CATCH_VOID
}
ELG_EXPORT Path* elg_path_stroked(const Path* path, const ElgStrokeStyle* style, float tolerance) {
    ELG_TRY
    return new Path(path->stroked(stroke_style(*style), tolerance));
    ELG_CATCH(nullptr)
}

// --- Drawing ---

// `draw` (args: GraphicsContext*) runs every time the view is drawn.
ELG_EXPORT View* elg_canvas(ElgClosure draw) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(draw);
    return out(Canvas([f](GraphicsContext& context) { f->call(&context, nullptr); }));
    ELG_CATCH(nullptr)
}
ELG_EXPORT ElgSize elg_gc_size(const GraphicsContext* gc) {
    const Size s = gc->size();
    return {s.width, s.height};
}
ELG_EXPORT float elg_gc_scale(const GraphicsContext* gc) {
    return gc->scale();
}
ELG_EXPORT double elg_gc_time(const GraphicsContext* gc) {
    return gc->time();
}
ELG_EXPORT void elg_gc_request_frame(GraphicsContext* gc) {
    gc->request_frame();
}
ELG_EXPORT void elg_gc_fill_rect(GraphicsContext* gc, ElgRect r, ElgColor c, const float* radii) {
    ELG_TRY
    gc->fill_rect(rect(r), color(c), CornerRadii{radii[0], radii[1], radii[2], radii[3]});
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_gc_stroke_rect(GraphicsContext* gc, ElgRect r, ElgColor c, float width, const float* radii) {
    ELG_TRY
    gc->stroke_rect(rect(r), color(c), width, CornerRadii{radii[0], radii[1], radii[2], radii[3]});
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_gc_draw_rect(GraphicsContext* gc, ElgRect r, const float* radii, ElgColor fill, ElgColor border,
                                 float border_width) {
    ELG_TRY
    gc->draw_rect(rect(r), CornerRadii{radii[0], radii[1], radii[2], radii[3]}, color(fill), color(border), border_width);
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_gc_fill_path(GraphicsContext* gc, const Path* path, const ElgShapeStyle* style, int32_t rule) {
    ELG_TRY
    gc->fill_path(*path, shape_style(*style), static_cast<FillRule>(rule));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_gc_stroke_path(GraphicsContext* gc, const Path* path, ElgColor c, const ElgStrokeStyle* style) {
    ELG_TRY
    gc->stroke_path(*path, color(c), stroke_style(*style));
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_gc_push_clip(GraphicsContext* gc, ElgRect r, const float* radii) {
    ELG_TRY
    gc->push_clip(rect(r), CornerRadii{radii[0], radii[1], radii[2], radii[3]});
    ELG_CATCH_VOID
}
ELG_EXPORT void elg_gc_pop_clip(GraphicsContext* gc) {
    ELG_TRY
    gc->pop_clip();
    ELG_CATCH_VOID
}

// --- Images ---

ELG_EXPORT View* elg_image(const ElgImageSource* source) {
    VIEW_TRY(Image(image_source(*source)))
}

ELG_EXPORT View* elg_image_named(ElgStr system_name) {
    VIEW_TRY(Image(view(system_name)))
}

// content_mode < 0: none.
static AsyncImageOptions async_image_options(float scale, int32_t content_mode, bool disk_cache) {
    AsyncImageOptions options;
    options.scale = scale;
    if (content_mode >= 0) {
        options.content_mode = static_cast<ContentMode>(content_mode);
    }
    options.disk_cache = disk_cache;
    return options;
}

ELG_EXPORT View* elg_async_image(ElgStr url, float scale, int32_t content_mode, bool disk_cache) {
    VIEW_TRY(AsyncImage(str(url), async_image_options(scale, content_mode, disk_cache)))
}

// directory empty: the default.
ELG_EXPORT void elg_set_image_cache(bool disk, uint64_t disk_limit, ElgStr directory) {
    ELG_TRY
    set_image_cache({disk, disk_limit, path(directory)});
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_clear_image_cache() {
    ELG_TRY
    clear_image_cache();
    ELG_CATCH_VOID
}

struct ElgAsyncImagePhase {
    int32_t kind;
    const View* image;
    ElgStr error;
};

// `content` (args: ElgAsyncImagePhase*, ret: View*) builds the view for each phase.
ELG_EXPORT View* elg_async_image_content(ElgStr url, ElgClosure content, float scale, bool disk_cache) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(content);
    return out(AsyncImage(
        str(url),
        [f](const AsyncImagePhase& phase) {
            ElgAsyncImagePhase arg{static_cast<int32_t>(phase.kind), &phase.image, c_str(phase.error)};
            View result;
            f->call(&arg, &result);
            return result;
        },
        async_image_options(scale, -1, disk_cache)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_label(ElgProp title, const ElgImageSource* icon) {
    VIEW_TRY(Label(prop<std::string>(title), image_source(*icon)))
}

struct ElgIconFont {
    ElgStr file;
    ElgBytes data;
};

static IconFont icon_font(const ElgIconFont& f) {
    if (f.data.ptr) {
        return IconFont(bytes(f.data));
    }
    if (f.file.len) {
        return IconFont(path(f.file));
    }
    return IconFont();
}

struct ElgIconWeight {
    int32_t weight;
    ElgIconFont font;
};

struct ElgIconVariant {
    ElgStr name;
    ElgIconFont font;
    ElgStr names;
    ElgStr names_data;
};

ELG_EXPORT void elg_register_icon_set(ElgStr name, ElgIconFont font, const ElgIconWeight* weights, size_t weight_count,
                                      const ElgIconVariant* variants, size_t variant_count, ElgStr names,
                                      ElgStr names_data, bool replace) {
    ELG_TRY
    IconSetOptions options;
    options.font = icon_font(font);
    for (size_t i = 0; i < weight_count; ++i) {
        options.weights.emplace_back(static_cast<Weight>(weights[i].weight), icon_font(weights[i].font));
    }
    for (size_t i = 0; i < variant_count; ++i) {
        options.variants.push_back(
            {str(variants[i].name), icon_font(variants[i].font), path(variants[i].names), view(variants[i].names_data)});
    }
    options.names = path(names);
    options.names_data = view(names_data);
    options.replace = replace;
    register_icon_set(str(name), std::move(options));
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_set_default_icon_set(ElgStr name) {
    ELG_TRY
    set_default_icon_set(str(name));
    ELG_CATCH_VOID
}

// --- Fonts ---

ELG_EXPORT void elg_register_font_file(ElgStr family, ElgStr file, int32_t weight) {
    ELG_TRY
    register_font(str(family), path(file), static_cast<Weight>(weight));
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_register_font_data(ElgStr family, ElgBytes data, int32_t weight) {
    ELG_TRY
    register_font(str(family), bytes(data), static_cast<Weight>(weight));
    ELG_CATCH_VOID
}

ELG_EXPORT void elg_register_emoji_font(ElgStr file) {
    ELG_TRY
    register_emoji_font(path(file));
    ELG_CATCH_VOID
}

// Writes a font into a closure's Font result.
ELG_EXPORT void elg_font_assign(Font* target, const ElgFont* value) {
    *target = font(*value);
}

ELG_EXPORT void elg_shape_style_assign(ShapeStyle* target, const ElgShapeStyle* value) {
    *target = shape_style(*value);
}

// --- View modifiers ---

#define MOD_TRY(...)                  \
    ELG_TRY return out(__VA_ARGS__); \
    ELG_CATCH(nullptr)

ELG_EXPORT View* elg_mod_padding(const View* v, ElgProp insets) {
    MOD_TRY(v->padding(prop<EdgeInsets>(insets)))
}
// edges: Edges.
ELG_EXPORT View* elg_mod_padding_edges(const View* v, uint32_t edges, ElgProp length) {
    MOD_TRY(v->padding(static_cast<Edges>(edges), prop<float>(length)))
}

struct ElgFrameOptions {
    ElgOptLen width, height;
    ElgOptLen min_width, ideal_width, max_width;
    ElgOptLen min_height, ideal_height, max_height;
    ElgAlignment alignment;
};

ELG_EXPORT View* elg_mod_frame(const View* v, const ElgFrameOptions* o) {
    ELG_TRY
    const bool flexible = o->min_width.has || o->ideal_width.has || o->max_width.has || o->min_height.has ||
                          o->ideal_height.has || o->max_height.has;
    if (!flexible) {
        return out(v->frame(FrameOptions{length(o->width), length(o->height), alignment(o->alignment)}));
    }
    // A fixed dimension in a flexible frame: the same min, ideal, and max.
    FlexibleFrameOptions f;
    f.min_width = o->width.has ? length(o->width) : length(o->min_width);
    f.ideal_width = o->width.has ? length(o->width) : length(o->ideal_width);
    f.max_width = o->width.has ? length(o->width) : length(o->max_width);
    f.min_height = o->height.has ? length(o->height) : length(o->min_height);
    f.ideal_height = o->height.has ? length(o->height) : length(o->ideal_height);
    f.max_height = o->height.has ? length(o->height) : length(o->max_height);
    f.alignment = alignment(o->alignment);
    return out(v->frame(f));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_mod_frame_size(const View* v, ElgProp size, ElgAlignment a) {
    MOD_TRY(v->frame(prop<Size>(size), alignment(a)))
}
ELG_EXPORT View* elg_mod_fixed_size(const View* v, bool horizontal, bool vertical) {
    MOD_TRY(v->fixed_size(horizontal, vertical))
}
ELG_EXPORT View* elg_mod_aspect_ratio(const View* v, bool has_ratio, float ratio, int32_t mode) {
    ELG_TRY
    if (has_ratio) {
        return out(v->aspect_ratio(ratio, static_cast<ContentMode>(mode)));
    }
    return out(v->aspect_ratio(static_cast<ContentMode>(mode)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_resizable(const View* v) {
    MOD_TRY(v->resizable())
}
ELG_EXPORT View* elg_mod_layout_priority(const View* v, float priority) {
    MOD_TRY(v->layout_priority(priority))
}
ELG_EXPORT View* elg_mod_z_index(const View* v, ElgProp z) {
    MOD_TRY(v->z_index(prop<float>(z)))
}
ELG_EXPORT View* elg_mod_offset(const View* v, ElgProp x, ElgProp y) {
    MOD_TRY(v->offset(prop<float>(x), prop<float>(y)))
}
ELG_EXPORT View* elg_mod_background_view(const View* v, View* take_background, ElgAlignment a) {
    MOD_TRY(v->background(take(take_background), alignment(a)))
}
ELG_EXPORT View* elg_mod_background(const View* v, ElgProp style) {
    MOD_TRY(v->background(prop<ShapeStyle>(style)))
}
ELG_EXPORT View* elg_mod_background_shape(const View* v, ElgProp style, const Shape* shape) {
    MOD_TRY(v->background(prop<ShapeStyle>(style), *shape))
}
ELG_EXPORT View* elg_mod_overlay(const View* v, View* take_overlay, ElgAlignment a) {
    MOD_TRY(v->overlay(take(take_overlay), alignment(a)))
}
// shape null: the view's bounds.
ELG_EXPORT View* elg_mod_border(const View* v, ElgProp c, float width, const Shape* shape) {
    ELG_TRY
    if (shape) {
        return out(v->border(prop<Color>(c), width, *shape));
    }
    return out(v->border(prop<Color>(c), width));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_clipped(const View* v) {
    MOD_TRY(v->clipped())
}
ELG_EXPORT View* elg_mod_clip_shape(const View* v, const Shape* shape) {
    MOD_TRY(v->clip_shape(*shape))
}
ELG_EXPORT View* elg_mod_corner_radius(const View* v, float radius) {
    MOD_TRY(v->corner_radius(radius))
}
ELG_EXPORT View* elg_mod_opacity(const View* v, ElgProp opacity) {
    MOD_TRY(v->opacity(prop<float>(opacity)))
}
ELG_EXPORT View* elg_mod_shadow(const View* v, ElgShadow s) {
    MOD_TRY(v->shadow(shadow(s)))
}
ELG_EXPORT View* elg_mod_inner_shadow(const View* v, ElgShadow s) {
    MOD_TRY(v->inner_shadow(shadow(s)))
}
ELG_EXPORT View* elg_mod_blur(const View* v, ElgProp radius, bool opaque) {
    MOD_TRY(v->blur(prop<float>(radius), opaque))
}
// shape null: the view's bounds.
ELG_EXPORT View* elg_mod_backdrop_blur(const View* v, ElgProp radius, const Shape* shape, ElgProp saturation,
                                       ElgProp brightness, ElgProp noise) {
    ELG_TRY
    BackdropOptions options{prop<float>(saturation), prop<float>(brightness), prop<float>(noise)};
    if (shape) {
        return out(v->backdrop_blur(prop<float>(radius), *shape, std::move(options)));
    }
    return out(v->backdrop_blur(prop<float>(radius), std::move(options)));
    ELG_CATCH(nullptr)
}
// kind: ColorFilter::Kind. `amount` (unused for multiply, luminance_to_alpha), `color` (multiply only).
ELG_EXPORT View* elg_mod_color_filter(const View* v, int32_t kind, ElgProp amount, ElgProp c) {
    ELG_TRY
    ColorFilter filter;
    filter.kind = static_cast<ColorFilter::Kind>(kind);
    filter.amount = prop<float>(amount);
    filter.color = prop<Color>(c);
    return out(v->modifier(ColorFilterModifier(std::move(filter))));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_blend_mode(const View* v, int32_t mode) {
    MOD_TRY(v->blend_mode(static_cast<BlendMode>(mode)))
}
ELG_EXPORT View* elg_mod_compositing_group(const View* v) {
    MOD_TRY(v->compositing_group())
}
ELG_EXPORT View* elg_mod_rotation_effect(const View* v, ElgProp degrees, ElgPoint anchor) {
    MOD_TRY(v->rotation_effect(prop<float>(degrees), unit_point(anchor)))
}
// `y` null: a uniform scale by `x`.
ELG_EXPORT View* elg_mod_scale_effect(const View* v, ElgProp x, const ElgProp* y, ElgPoint anchor) {
    ELG_TRY
    if (!y) {
        return out(v->scale_effect(prop<float>(x), unit_point(anchor)));
    }
    return out(v->scale_effect(prop<float>(x), prop<float>(*y), unit_point(anchor)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_mask(const View* v, View* take_mask, ElgAlignment a) {
    MOD_TRY(v->mask(take(take_mask), alignment(a)))
}
ELG_EXPORT View* elg_mod_mask_style(const View* v, ElgProp style) {
    MOD_TRY(v->mask(prop<ShapeStyle>(style)))
}
ELG_EXPORT View* elg_mod_content_shape(const View* v, const Shape* shape) {
    MOD_TRY(v->content_shape(*shape))
}
ELG_EXPORT View* elg_mod_allows_hit_testing(const View* v, bool enabled) {
    MOD_TRY(v->allows_hit_testing(enabled))
}
// `on_click` args: ElgClickEvent*.
ELG_EXPORT View* elg_mod_on_click(const View* v, ElgClosure on_click, int32_t button, int32_t count) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(on_click);
    return out(v->on_click(
        [f](const ClickEvent& event) {
            ElgClickEvent arg = c_click_event(event);
            f->call(&arg, nullptr);
        },
        ClickOptions{static_cast<MouseButton>(button), count}));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_on_activate(const View* v, ElgClosure on_activate) {
    MOD_TRY(v->on_activate(action(on_activate)))
}
ELG_EXPORT View* elg_mod_on_size_change(const View* v, ElgClosure on_size_change) {
    ELG_TRY
    SharedFn f = share(on_size_change);
    return out(v->on_size_change([f](PixelSize size) {
        ElgPixelSize arg{size.width, size.height};
        f->call(&arg, nullptr);
    }));
    ELG_CATCH(nullptr)
}

// --- Gestures ---

// A callback taking a C event `C` made from the C++ event `E` by `convert`; empty if none.
template <class E, class C>
static std::function<void(const E&)> event_callback(ElgClosure closure, C (*convert)(const E&)) {
    SharedFn f = share(closure);
    if (!f) {
        return {};
    }
    return [f, convert](const E& event) {
        C arg = convert(event);
        f->call(&arg, nullptr);
    };
}

struct ElgGestureOptions {
    int32_t priority; // GesturePriority
    bool simultaneous;
};

static GestureOptions gesture_options(ElgGestureOptions o) {
    return {static_cast<GesturePriority>(o.priority), o.simultaneous};
}

struct ElgDragOptions {
    float minimum_distance;
    int32_t button;
    double hold;
    float hold_tolerance;
};

// began, changed, ended: args ElgDragEvent*; cancelled: no args. Any may be none.
ELG_EXPORT View* elg_mod_drag_gesture(const View* v, ElgDragOptions o, ElgClosure began, ElgClosure changed,
                                      ElgClosure ended, ElgClosure cancelled, ElgGestureOptions options) {
    ELG_TRY
    DragGesture gesture({o.minimum_distance, static_cast<MouseButton>(o.button), o.hold, o.hold_tolerance});
    if (auto f = event_callback(began, &c_drag_event)) {
        gesture = gesture.on_began(std::move(f));
    }
    if (auto f = event_callback(changed, &c_drag_event)) {
        gesture = gesture.on_changed(std::move(f));
    }
    if (auto f = event_callback(ended, &c_drag_event)) {
        gesture = gesture.on_ended(std::move(f));
    }
    if (auto f = action(cancelled)) {
        gesture = gesture.on_cancelled(std::move(f));
    }
    return out(v->recognize(std::move(gesture), gesture_options(options)));
    ELG_CATCH(nullptr)
}

struct ElgLongPressOptions {
    double duration;
    float maximum_distance;
    int32_t button;
};

// pressing: args bool*, may be none.
ELG_EXPORT View* elg_mod_long_press_gesture(const View* v, ElgClosure on_press, ElgLongPressOptions o, ElgClosure pressing,
                                           ElgGestureOptions options) {
    ELG_TRY
    LongPressOptions long_press{o.duration, o.maximum_distance, static_cast<MouseButton>(o.button), consumer<bool>(pressing)};
    return out(v->recognize(LongPressGesture(action(on_press), std::move(long_press)), gesture_options(options)));
    ELG_CATCH(nullptr)
}

template <class Event, class C>
static PhasedGesture<Event> phased_gesture(ElgClosure began, ElgClosure changed, ElgClosure ended, ElgClosure cancelled,
                                           C (*convert)(const Event&)) {
    PhasedGesture<Event> gesture;
    if (auto f = event_callback(began, convert)) {
        gesture = gesture.on_began(std::move(f));
    }
    if (auto f = event_callback(changed, convert)) {
        gesture = gesture.on_changed(std::move(f));
    }
    if (auto f = event_callback(ended, convert)) {
        gesture = gesture.on_ended(std::move(f));
    }
    if (auto f = action(cancelled)) {
        gesture = gesture.on_cancelled(std::move(f));
    }
    return gesture;
}

// began, changed, ended: args ElgMagnifyEvent*; cancelled: no args.
ELG_EXPORT View* elg_mod_magnify_gesture(const View* v, ElgClosure began, ElgClosure changed, ElgClosure ended,
                                         ElgClosure cancelled) {
    MOD_TRY(v->recognize(phased_gesture(began, changed, ended, cancelled, &c_magnify_event)))
}
// began, changed, ended: args ElgRotateEvent*; cancelled: no args.
ELG_EXPORT View* elg_mod_rotate_gesture(const View* v, ElgClosure began, ElgClosure changed, ElgClosure ended,
                                        ElgClosure cancelled) {
    MOD_TRY(v->recognize(phased_gesture(began, changed, ended, cancelled, &c_rotate_event)))
}
// `action` args: ElgPoint* (window coordinates).
ELG_EXPORT View* elg_mod_on_two_finger_double_tap(const View* v, ElgClosure on_tap) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(on_tap);
    return out(v->on_two_finger_double_tap([f](Point location) {
        ElgPoint arg = c_point(location);
        f->call(&arg, nullptr);
    }));
    ELG_CATCH(nullptr)
}

// --- Pointer, scrolling, keys, text ---

ELG_EXPORT View* elg_mod_pointer_style(const View* v, ElgProp style) {
    MOD_TRY(v->pointer_style(prop<PointerStyle>(style)))
}
// leave: on_pointer_leave, else on_pointer_enter.
ELG_EXPORT View* elg_mod_on_pointer_boundary(const View* v, bool leave, ElgClosure on_cross) {
    ELG_TRY
    if (leave) {
        return out(v->on_pointer_leave(action(on_cross)));
    }
    return out(v->on_pointer_enter(action(on_cross)));
    ELG_CATCH(nullptr)
}
// `on_hover` args: bool*.
ELG_EXPORT View* elg_mod_on_hover_change(const View* v, ElgClosure on_hover) {
    MOD_TRY(v->on_hover_change(consumer<bool>(on_hover)))
}
// `on_pointer` args: ElgPointerEvent* (view-local), ret: bool* (consumed).
ELG_EXPORT View* elg_mod_on_pointer(const View* v, ElgClosure on_pointer) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(on_pointer);
    return out(v->on_pointer([f](const PointerEvent& event) {
        ElgPointerEvent arg = c_pointer_event(event);
        bool consumed = false;
        f->call(&arg, &consumed);
        return consumed;
    }));
    ELG_CATCH(nullptr)
}
// `on_scroll` args: ElgScrollEvent* (view-local), ret: bool* (consumed).
ELG_EXPORT View* elg_mod_on_scroll(const View* v, ElgClosure on_scroll) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(on_scroll);
    return out(v->on_scroll([f](const ScrollEvent& event) {
        ElgScrollEvent arg = c_scroll_event(event);
        bool consumed = false;
        f->call(&arg, &consumed);
        return consumed;
    }));
    ELG_CATCH(nullptr)
}
// `on_text_input` args: ElgTextInputEvent*, ret: bool* (consumed).
ELG_EXPORT View* elg_mod_on_text_input(const View* v, ElgClosure on_text_input) {
    ELG_TRY
    SharedFn f = std::make_shared<const RustFn>(on_text_input);
    return out(v->on_text_input([f](const TextInputEvent& event) {
        ElgTextInputEvent arg{static_cast<int32_t>(event.kind), c_str(event.text), event.cursor};
        bool consumed = false;
        f->call(&arg, &consumed);
        return consumed;
    }));
    ELG_CATCH(nullptr)
}

struct ElgDropFiles {
    const ElgStr* paths;
    size_t count;
    ElgPoint location;
};

struct ElgDropText {
    ElgStr text;
    ElgPoint location;
};

// files (args: ElgDropFiles*, ret: bool*), text (args: ElgDropText*, ret: bool*), targeted (args: bool*).
ELG_EXPORT View* elg_mod_on_drop(const View* v, ElgClosure files, ElgClosure text, ElgClosure targeted) {
    ELG_TRY
    DropOptions options;
    if (SharedFn f = share(files)) {
        options.files = [f](const std::vector<std::filesystem::path>& paths, Point location) {
            std::vector<std::string> names;
            names.reserve(paths.size());
            for (const auto& p : paths) {
                names.push_back(utf8(p));
            }
            std::vector<ElgStr> c_names;
            c_names.reserve(names.size());
            for (const auto& n : names) {
                c_names.push_back(c_str(n));
            }
            ElgDropFiles arg{c_names.data(), c_names.size(), {location.x, location.y}};
            bool wanted = false;
            f->call(&arg, &wanted);
            return wanted;
        };
    }
    if (SharedFn f = share(text)) {
        options.text = [f](const std::string& dropped, Point location) {
            ElgDropText arg{c_str(dropped), {location.x, location.y}};
            bool wanted = false;
            f->call(&arg, &wanted);
            return wanted;
        };
    }
    options.targeted = consumer<bool>(targeted);
    return out(v->on_drop(std::move(options)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_mod_accessibility_label(const View* v, ElgProp label) {
    MOD_TRY(v->accessibility_label(prop<std::string>(label)))
}
ELG_EXPORT View* elg_mod_accessibility_value(const View* v, ElgProp value) {
    MOD_TRY(v->accessibility_value(prop<std::string>(value)))
}
ELG_EXPORT View* elg_mod_accessibility_hint(const View* v, ElgProp hint) {
    MOD_TRY(v->accessibility_hint(prop<std::string>(hint)))
}
ELG_EXPORT View* elg_mod_accessibility_hidden(const View* v, bool hidden) {
    MOD_TRY(v->accessibility_hidden(hidden))
}
ELG_EXPORT View* elg_mod_accessibility_role(const View* v, int32_t role) {
    MOD_TRY(v->accessibility_role(static_cast<AccessibilityRole>(role)))
}
ELG_EXPORT View* elg_mod_accessibility_element(const View* v) {
    MOD_TRY(v->accessibility_element())
}
ELG_EXPORT View* elg_mod_accessibility_action(const View* v, ElgClosure on_action) {
    MOD_TRY(v->accessibility_action(action(on_action)))
}
ELG_EXPORT View* elg_mod_hover_delay(const View* v, ElgHoverDelay delay) {
    MOD_TRY(v->hover_delay(hover_delay(delay)))
}
ELG_EXPORT View* elg_mod_window_drag_area(const View* v, bool enabled) {
    MOD_TRY(v->window_drag_area(enabled))
}
ELG_EXPORT View* elg_mod_context_menu(const View* v, ElgClosure menu) {
    MOD_TRY(v->context_menu(view_builder(menu)))
}
struct ElgFloatingOptions {
    ElgFloatingPlacement placement;
    int32_t boundary; // FloatingBoundary
    bool outside_click;
    bool escape;
    ElgPopupMotion motion;
};

ELG_EXPORT View* elg_mod_floating(const View* v, Binding<bool>* take_is_presented, ElgClosure content,
                                  const ElgFloatingOptions* o) {
    ELG_TRY
    std::unique_ptr<Binding<bool>> is_presented(take_is_presented);
    FloatingOptions options;
    options.placement = placement(o->placement);
    options.boundary = static_cast<FloatingBoundary>(o->boundary);
    options.dismiss = {o->outside_click, o->escape};
    options.motion = motion(o->motion);
    return out(v->floating(*is_presented, view_builder(content), std::move(options)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_floating_container(const View* v) {
    MOD_TRY(v->floating_container())
}

struct ElgTooltipOptions {
    ElgFloatingPlacement placement;
    int32_t boundary;
    double delay;
    ElgPopupMotion motion;
};

static TooltipOptions tooltip_options(const ElgTooltipOptions& o) {
    return {placement(o.placement), static_cast<FloatingBoundary>(o.boundary), o.delay, motion(o.motion)};
}

// One of `content` (ret: View*) and `text`.
ELG_EXPORT View* elg_mod_tooltip(const View* v, ElgClosure content, const ElgProp* text, const ElgTooltipOptions* o) {
    ELG_TRY
    if (text) {
        return out(v->tooltip(prop<std::string>(*text), tooltip_options(*o)));
    }
    return out(v->tooltip(view_builder(content), tooltip_options(*o)));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_mod_popover(const View* v, Binding<bool>* take_is_presented, ElgClosure content,
                                 const ElgFloatingPlacement* popover_placement, int32_t boundary, ElgProp padded,
                                 ElgPopupMotion popup_motion) {
    ELG_TRY
    std::unique_ptr<Binding<bool>> is_presented(take_is_presented);
    PopoverOptions options;
    options.placement = placement(*popover_placement);
    options.boundary = static_cast<FloatingBoundary>(boundary);
    options.padded = prop<bool>(padded);
    options.motion = motion(popup_motion);
    return out(v->popover(*is_presented, view_builder(content), std::move(options)));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_focusable(const View* v, bool focusable) {
    MOD_TRY(v->focusable(focusable))
}
ELG_EXPORT View* elg_mod_focused(const View* v, Binding<bool>* take_is_focused) {
    ELG_TRY
    std::unique_ptr<Binding<bool>> is_focused(take_is_focused);
    return out(v->focused(*is_focused));
    ELG_CATCH(nullptr)
}
ELG_EXPORT View* elg_mod_on_key_press(const View* v, int32_t key, ElgClosure on_press) {
    MOD_TRY(v->on_key_press(static_cast<Key>(key), action(on_press)))
}
// `on_key` args: ElgKeyEvent* (downs and ups), ret: bool* (handled).
ELG_EXPORT View* elg_mod_on_key(const View* v, ElgClosure on_key) {
    MOD_TRY(v->on_key(key_handler(on_key)))
}
// --- Selection ---

struct ElgItemKey {
    bool is_string;
    int64_t int_key;
    ElgStr string_key;
};

struct ElgItemKeys {
    const ElgItemKey* keys;
    size_t count;
};

ELG_EXPORT void elg_item_keys_push(std::vector<ItemKey>* target, ElgItemKey key) {
    target->push_back(key.is_string ? ItemKey(str(key.string_key)) : ItemKey(key.int_key));
}

namespace {

ElgItemKey c_item_key(const ItemKey& key) {
    if (const auto* s = std::get_if<std::string>(&key)) {
        return {true, 0, c_str(*s)};
    }
    return {false, std::get<std::int64_t>(key), {}};
}

std::vector<ItemKey> read_keys(const RustFn& get) {
    std::vector<ItemKey> keys;
    get.call(nullptr, &keys);
    return keys;
}

void write_keys(const RustFn& set, const std::vector<ItemKey>& keys) {
    std::vector<ElgItemKey> items;
    items.reserve(keys.size());
    for (const ItemKey& key : keys) {
        items.push_back(c_item_key(key));
    }
    ElgItemKeys arg{items.data(), items.size()};
    set.call(&arg, nullptr);
}

// View::selection over the Rust selection, with keys of type K (std::int64_t or std::string).
template <class K>
View with_selection(const View& v, bool multiple, SharedFn get, SharedFn set, SharedFn activate) {
    SelectionOptions<K> options;
    if (activate) {
        options.on_activate = [activate](const K& key) {
            const ItemKey item = detail::to_item_key(key);
            ElgItemKey arg = c_item_key(item);
            activate->call(&arg, nullptr);
        };
    }
    if (multiple) {
        Binding<std::set<K>> binding(
            [get] {
                std::set<K> keys;
                for (const ItemKey& key : read_keys(*get)) {
                    keys.insert(detail::from_item_key<K>(key));
                }
                return keys;
            },
            [set](std::set<K> keys) {
                std::vector<ItemKey> items;
                for (const K& key : keys) {
                    items.push_back(detail::to_item_key(key));
                }
                write_keys(*set, items);
            });
        return v.selection(std::move(binding), std::move(options));
    }
    Binding<std::optional<K>> binding(
        [get]() -> std::optional<K> {
            const std::vector<ItemKey> keys = read_keys(*get);
            return keys.empty() ? std::nullopt : std::optional<K>(detail::from_item_key<K>(keys.front()));
        },
        [set](std::optional<K> key) {
            write_keys(*set, key ? std::vector<ItemKey>{detail::to_item_key(*key)} : std::vector<ItemKey>{});
        });
    return v.selection(std::move(binding), std::move(options));
}

} // namespace

// get (ret: std::vector<ItemKey>*, filled with elg_item_keys_push) is a tracked read; set (args:
// ElgItemKeys*); activate (args: ElgItemKey*) may be none. string_keys: the ForEach's keys are strings.
ELG_EXPORT View* elg_mod_selection(const View* v, bool multiple, bool string_keys, ElgClosure get, ElgClosure set,
                                   ElgClosure activate) {
    ELG_TRY
    SharedFn g = std::make_shared<const RustFn>(get);
    SharedFn s = std::make_shared<const RustFn>(set);
    SharedFn a = share(activate);
    if (string_keys) {
        return out(with_selection<std::string>(*v, multiple, g, s, a));
    }
    return out(with_selection<std::int64_t>(*v, multiple, g, s, a));
    ELG_CATCH(nullptr)
}

ELG_EXPORT View* elg_mod_font_weight(const View* v, ElgProp weight) {
    MOD_TRY(v->font_weight(prop<Weight>(weight)))
}
ELG_EXPORT View* elg_mod_font(const View* v, ElgProp f) {
    MOD_TRY(v->font(prop<Font>(f)))
}
ELG_EXPORT View* elg_mod_foreground_style(const View* v, ElgProp c) {
    MOD_TRY(v->foreground_style(prop<Color>(c)))
}
ELG_EXPORT View* elg_mod_tint(const View* v, ElgProp c) {
    MOD_TRY(v->tint(prop<Color>(c)))
}
ELG_EXPORT View* elg_mod_color_scheme(const View* v, ElgProp scheme) {
    MOD_TRY(v->color_scheme(prop<ColorScheme>(scheme)))
}
ELG_EXPORT View* elg_mod_theme(const View* v, const Theme* theme) {
    MOD_TRY(v->theme(*theme))
}
ELG_EXPORT View* elg_mod_disabled(const View* v, ElgProp disabled) {
    MOD_TRY(v->disabled(prop<bool>(disabled)))
}
namespace {
// View::animation's source: Rust returns a version number that changes when its value does.
struct VersionSource {
    SharedFn f;
    std::uint64_t operator()() const {
        std::uint64_t version = 0;
        f->call(nullptr, &version);
        return version;
    }
};
} // namespace

ELG_EXPORT View* elg_mod_animation(const View* v, const Animation* animation, ElgClosure version) {
    MOD_TRY(v->animation(*animation, VersionSource{std::make_shared<const RustFn>(version)}))
}
ELG_EXPORT View* elg_mod_transition(const View* v, const Transition* transition) {
    MOD_TRY(v->transition(*transition))
}
ELG_EXPORT View* elg_mod_animates_layout(const View* v, bool enabled) {
    MOD_TRY(v->animates_layout(enabled))
}

// --- Hot reload ---

ELG_EXPORT View* elg_hot_reload(ElgStr module, ElgStr build_command, const ElgStr* watch, size_t watch_count, int32_t show) {
    ELG_TRY
    HotReloadOptions options;
    options.module = path(module);
    options.build_command = str(build_command);
    for (size_t i = 0; i < watch_count; ++i) {
        options.watch.push_back(path(watch[i]));
    }
    options.show = static_cast<HotReloadShow>(show);
    return out(HotReload(std::move(options)));
    ELG_CATCH(nullptr)
}

// --- Views modules (hot reload and previews) ---
//
// A Rust views module exports elg_module_entry (elegance's `preview` feature), which calls this. The entry
// holds plain `View (*)()` functions, so each preview gets a thunk from a fixed table that calls
// back into Rust with its index (-1: the root view).

struct ElgPreviewDesc {
    ElgStr name;
    ElgStr file;
    int32_t line;
};

// Builds view `index` into `ret`; false if it panicked.
typedef bool (*ElgModuleBuild)(int32_t index, View* ret);

namespace {

constexpr std::size_t kMaxPreviews = 256;

ElgModuleBuild g_module_build = nullptr;
std::vector<std::string> g_preview_strings;
std::vector<PreviewEntry> g_previews;
ModuleEntry g_module_entry;

View build_module_view(int32_t index) {
    View result;
    if (!g_module_build(index, &result)) {
        throw RustPanic();
    }
    return result;
}

View module_root() {
    return build_module_view(-1);
}

template <std::size_t... I>
std::array<View (*)(), sizeof...(I)> make_preview_thunks(std::index_sequence<I...>) {
    return {+[]() -> View { return build_module_view(static_cast<int32_t>(I)); }...};
}

const std::array<View (*)(), kMaxPreviews> g_preview_thunks = make_preview_thunks(std::make_index_sequence<kMaxPreviews>{});

} // namespace

ELG_EXPORT const ModuleEntry* elg_rust_module_entry(bool has_root, const ElgPreviewDesc* previews, size_t count,
                                                    ElgModuleBuild build) {
    static std::once_flag once;
    std::call_once(once, [&] {
        g_module_build = build;
        count = std::min(count, kMaxPreviews);
        g_preview_strings.reserve(count * 2); // stable c_str()s
        g_previews.reserve(count);
        for (size_t i = 0; i < count; ++i) {
            const char* name = g_preview_strings.emplace_back(str(previews[i].name)).c_str();
            const char* file = g_preview_strings.emplace_back(str(previews[i].file)).c_str();
            g_previews.push_back({name, g_preview_thunks[i], file, previews[i].line});
        }
        g_module_entry = {module_abi, has_root ? &module_root : nullptr, &g_previews};
    });
    return &g_module_entry;
}

// --- Grids ---

struct ElgGridItem {
    int32_t kind; // GridItem::Kind
    float minimum;
    float maximum;
    ElgOptLen spacing;
    bool has_alignment;
    ElgAlignment alignment;
};

struct ElgGridTrack {
    float size;
    float spacing;
    bool has_alignment;
    ElgAlignment alignment;
};

// A VGrid/HGrid (kind 0/1: tracks `items`, `spacing_a` between lines) or a Grid (kind 2:
// `spacing_a` between columns, `spacing_b` between rows).
struct ElgGridDesc {
    int32_t kind;
    const ElgGridItem* items;
    size_t item_count;
    ElgAlignment alignment;
    ElgOptLen spacing_a;
    ElgOptLen spacing_b;
};

static std::vector<GridItem> grid_items(const ElgGridItem* items, size_t count) {
    std::vector<GridItem> result;
    result.reserve(count);
    for (size_t i = 0; i < count; ++i) {
        GridItem item{static_cast<GridItem::Kind>(items[i].kind), items[i].minimum, items[i].maximum, length(items[i].spacing)};
        if (items[i].has_alignment) {
            item.alignment = alignment(items[i].alignment);
        }
        result.push_back(item);
    }
    return result;
}

static std::shared_ptr<const Layout> grid_layout(const ElgGridDesc& d) {
    switch (d.kind) {
    case 0: return std::make_shared<const VGridLayout>(grid_items(d.items, d.item_count), alignment(d.alignment), length(d.spacing_a));
    case 1: return std::make_shared<const HGridLayout>(grid_items(d.items, d.item_count), alignment(d.alignment), length(d.spacing_a));
    default: return std::make_shared<const GridLayout>(alignment(d.alignment), length(d.spacing_a), length(d.spacing_b));
    }
}

ELG_EXPORT View* elg_grid(const ElgGridDesc* desc, View** children, size_t count) {
    ELG_TRY
    std::vector<View> views = take_all(children, count);
    switch (desc->kind) {
    case 0: return out(VGridLayout(grid_items(desc->items, desc->item_count), alignment(desc->alignment), length(desc->spacing_a))(std::move(views)));
    case 1: return out(HGridLayout(grid_items(desc->items, desc->item_count), alignment(desc->alignment), length(desc->spacing_a))(std::move(views)));
    default: return out(GridLayout(alignment(desc->alignment), length(desc->spacing_a), length(desc->spacing_b))(std::move(views)));
    }
    ELG_CATCH(nullptr)
}

ELG_EXPORT ElgSize elg_grid_layout_size(const ElgGridDesc* desc, ElgProposedSize proposal, const LayoutChild* children, size_t count) {
    ELG_TRY
    const Size s = grid_layout(*desc)->size_that_fits(proposed(proposal), std::span<const LayoutChild>(children, count));
    return {s.width, s.height};
    ELG_CATCH(ElgSize{})
}

ELG_EXPORT void elg_grid_layout_place(const ElgGridDesc* desc, ElgRect bounds, ElgProposedSize proposal,
                                      const LayoutChild* children, size_t count) {
    ELG_TRY
    grid_layout(*desc)->place_children(rect(bounds), proposed(proposal), std::span<const LayoutChild>(children, count));
    ELG_CATCH_VOID
}

// alignment: -1 for the grid's.
ELG_EXPORT View* elg_grid_row(int32_t row_alignment, View** cells, size_t count) {
    ELG_TRY
    GridRowOptions options;
    if (row_alignment >= 0) {
        options.alignment = static_cast<VerticalAlignment>(row_alignment);
    }
    return out(detail::make_grid_row(options, take_all(cells, count)));
    ELG_CATCH(nullptr)
}

// Calls `each` (args: ElgGridTrack*) per track.
ELG_EXPORT void elg_resolve_grid_tracks(const ElgGridItem* items, size_t count, bool has_length, float length_value,
                                        ElgOptLen spacing, ElgClosure each) {
    ELG_TRY
    const std::vector<GridItem> list = grid_items(items, count);
    const std::optional<float> span = has_length ? std::optional<float>(length_value) : std::nullopt;
    for (const GridTrack& track : resolve_grid_tracks(list, span, length(spacing))) {
        ElgGridTrack arg{track.size, track.spacing, track.alignment.has_value(), c_alignment(track.alignment.value_or(Alignment{}))};
        call_now(each, &arg, nullptr);
    }
    ELG_CATCH_VOID
}

ELG_EXPORT View* elg_mod_grid_cell_columns(const View* v, int32_t count) {
    MOD_TRY(v->grid_cell_columns(count))
}
ELG_EXPORT View* elg_mod_grid_cell_alignment(const View* v, ElgAlignment a) {
    MOD_TRY(v->grid_cell_alignment(alignment(a)))
}
ELG_EXPORT View* elg_mod_grid_column_alignment(const View* v, int32_t a) {
    MOD_TRY(v->grid_column_alignment(static_cast<HorizontalAlignment>(a)))
}

// --- ABI layout ---

ELG_EXPORT void elg_layouts_views(ElgStrSink out, void* ctx) {
    ELG_LAYOUT(ElgTextFieldOptions);
    ELG_LAYOUT(ElgNumberInputOptions);
    ELG_LAYOUT(ElgOptNumber);
    ELG_LAYOUT(ElgForEachItem);
    ELG_LAYOUT(ElgStackSeparators);
    ELG_LAYOUT(ElgSplitViewOptions);
    ELG_LAYOUT(ElgColumnWidth);
    ELG_LAYOUT(ElgNavigationSplitViewOptions);
    ELG_LAYOUT(ElgGestureOptions);
    ELG_LAYOUT(ElgDragOptions);
    ELG_LAYOUT(ElgLongPressOptions);
    ELG_LAYOUT(ElgFloatingOptions);
    ELG_LAYOUT(ElgTooltipOptions);
    ELG_LAYOUT(ElgItemKey);
    ELG_LAYOUT(ElgItemKeys);
    ELG_LAYOUT(ElgTextRun);
    ELG_LAYOUT(ElgPolyline);
    ELG_LAYOUT(ElgAsyncImagePhase);
    ELG_LAYOUT(ElgIconFont);
    ELG_LAYOUT(ElgIconWeight);
    ELG_LAYOUT(ElgIconVariant);
    ELG_LAYOUT(ElgFrameOptions);
    ELG_LAYOUT(ElgDropFiles);
    ELG_LAYOUT(ElgDropText);
    ELG_LAYOUT(ElgPreviewDesc);
    ELG_LAYOUT(ElgGridItem);
    ELG_LAYOUT(ElgGridTrack);
    ELG_LAYOUT(ElgGridDesc);
}
