//! Compile-time coverage for the API used by the DSL's receiver-based completion.
use elegance::dot::{self, DotTarget, DotValues, ResolveDotValues};
use elegance::prelude::*;

#[elg_builder]
impl CustomBuilder {
    pub fn pair(self, _: bool, _: impl Into<Alignment>) -> Self {
        self
    }
    pub fn concrete(self, _: Weight) -> Self {
        self
    }
    pub fn unknown<T>(self, _: impl Into<T>) -> Self {
        self
    }
}
struct CustomBuilder;

struct RegisteredComponent<T, const N: usize> {
    value: T,
    alignment: Alignment,
    constructor_ran: bool,
}

#[elg_builder(constructor)]
impl<T, const N: usize> RegisteredComponent<T, N>
where
    T: Default + 'static,
{
    pub fn new() -> Self {
        Self {
            value: T::default(),
            alignment: Alignment::CENTER,
            constructor_ran: true,
        }
    }

    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }
}

impl<T: 'static, const N: usize> Component for RegisteredComponent<T, N> {
    fn body(&self) -> impl IntoView {
        EmptyView()
    }
}

// The generated registration must be disabled along with either the impl or new.
#[elg_builder(constructor)]
#[cfg(any())]
impl MissingType {
    fn new() -> Self {
        unreachable!()
    }
}

struct DisabledConstructor;
#[elg_builder(constructor)]
impl DisabledConstructor {
    #[cfg_attr(all(), cfg(any()))]
    fn new() -> Self {
        Self
    }
}

#[test]
fn registered_struct_component_constructs_in_elg_without_manual_elg_new() {
    fn accepts_component(_: &impl Component) {}

    let component = elg!(RegisteredComponent::<String, 3>(alignment: .top_leading));
    accepts_component(&component);
    assert!(component.constructor_ran);
    assert_eq!(component.value, "");
    assert_eq!(component.alignment, Alignment::TOP_LEADING);
    let _ = DisabledConstructor;
    let default = elg!(RegisteredComponent::<u32, 1>());
    assert_eq!(default.value, 0);
    assert_eq!(default.alignment, Alignment::CENTER);
}

#[elg_builder]
trait CustomMethods {
    fn choose(self, _: impl Into<ContentMode>);
}
impl CustomMethods for CustomBuilder {
    fn choose(self, _: impl Into<ContentMode>) {}
}

#[component]
fn TypedProps(
    alignment: elegance::Alignment,
    #[prop(default)] weight: ::elegance::Weight,
) -> impl IntoView {
    let _ = (alignment, weight);
    EmptyView()
}

#[component]
fn ArbitraryProp(value: std::time::Duration) -> impl IntoView {
    let _ = value;
    EmptyView()
}

#[component]
fn UnqualifiedProps(alignment: Alignment) -> impl IntoView {
    let _ = alignment;
    EmptyView()
}

mod custom_props {
    use elegance::prelude::*;

    // These names intentionally shadow Elegance types and do not implement DotValues.
    #[derive(Clone, Default)]
    struct Color;
    #[derive(Clone, Default)]
    struct Axis;
    #[derive(Clone, Default)]
    struct Arbitrary;

    impl From<elegance::dot::white> for Color {
        fn from(_: elegance::dot::white) -> Self {
            Self
        }
    }

    impl From<elegance::dot::horizontal> for Axis {
        fn from(_: elegance::dot::horizontal) -> Self {
            Self
        }
    }

    #[component]
    fn CustomProps(color: Color, axis: Axis, data: Arbitrary) -> impl IntoView {
        let _ = (color, axis, data);
        EmptyView()
    }

    #[component]
    fn DefaultCustomProps(
        #[prop(default)] color: Color,
        #[prop(default)] axis: Axis,
        #[prop(default)] data: Arbitrary,
    ) -> impl IntoView {
        let _ = (color, axis, data);
        EmptyView()
    }

    #[allow(dead_code)]
    fn components_without_dot_values() {
        fn accepts_view(_: impl IntoView) {}

        accepts_view(CustomProps::new().color(Color).axis(Axis).data(Arbitrary));
        accepts_view(elg!(CustomProps(color: .white, axis: .horizontal, data: Arbitrary)));
        accepts_view(DefaultCustomProps::new());
        accepts_view(elg!(DefaultCustomProps(color: .white, axis: .horizontal)));
    }
}

// cargo check --tests checks these without invoking native UI constructors.
#[allow(dead_code)]
fn receiver_helpers() {
    let _ = CustomBuilder
        .pair(false, Alignment::CENTER)
        .concrete(Weight::Bold);
    CustomBuilder.choose(ContentMode::Fit);
    let custom = CustomBuilder;
    let _: Alignment = custom
        .__elg_dot_pair_1()
        .resolve_dot_values()
        .top_leading
        .into();
    let _: Weight = custom
        .__elg_dot_concrete_0()
        .resolve_dot_values()
        .bold
        .into();
    let _: ContentMode = custom.__elg_dot_choose_0().resolve_dot_values().fit.into();
    let _ = custom.unknown::<String>("value");

    let stack = VStack::new();
    let _: HorizontalAlignment = stack
        .__elg_dot_alignment_0()
        .resolve_dot_values()
        .leading
        .into();
    let _: Edges = stack
        .__elg_dot_padding_edges_0()
        .resolve_dot_values()
        .horizontal
        .into();
    let _: Len = stack
        .__elg_dot_frame_1()
        .resolve_dot_values()
        .infinity
        .into();
    let _: Prop<ShapeStyle> = stack
        .__elg_dot_background_0()
        .resolve_dot_values()
        .accent
        .into_prop();
    let _: Prop<ShapeStyle> = stack
        .__elg_dot_background_0()
        .resolve_dot_values()
        .thin_material
        .into_prop();
    let _: Prop<Color> = stack
        .__elg_dot_color_multiply_0()
        .resolve_dot_values()
        .white
        .into_prop();
    let _: Prop<Font> = stack
        .__elg_dot_font_0()
        .resolve_dot_values()
        .body
        .into_prop();
    let _: Prop<ColorScheme> = stack
        .__elg_dot_color_scheme_0()
        .resolve_dot_values()
        .dark
        .into_prop();

    let options = FrameOptions::default();
    let _: Len = options
        .__elg_dot_max_width_0()
        .resolve_dot_values()
        .infinity
        .into();
    let _: Alignment = options
        .__elg_dot_alignment_0()
        .resolve_dot_values()
        .center
        .into();
    let _: HorizontalAlignment = LazyVStack::default()
        .__elg_dot_alignment_0()
        .resolve_dot_values()
        .leading
        .into();
    let _: VerticalAlignment = LazyHStack::default()
        .__elg_dot_alignment_0()
        .resolve_dot_values()
        .bottom
        .into();
    let _: Len = LazyHStack::default()
        .__elg_dot_spacing_0()
        .resolve_dot_values()
        .infinity
        .into();

    let props = TypedProps::new();
    let alignment = props.__elg_dot_alignment_0().resolve_dot_values().center;
    let props = props.alignment(alignment);
    let weight = props.__elg_dot_weight_0().resolve_dot_values().semibold;
    let _ = props.weight(weight);
    let _ = ArbitraryProp::new().value(std::time::Duration::ZERO);
    let _ = elg!(ArbitraryProp(value: std::time::Duration::ZERO));
    let _ = elg!(TypedProps(alignment: .center, weight: .semibold));
    let _ = elg!(UnqualifiedProps(alignment: .center));
}

#[test]
fn namespaces_preserve_marker_types_and_defaults() {
    let horizontal = HorizontalAlignment::dot_values();
    let _: dot::leading = horizontal.leading;
    let _: dot::trailing = horizontal.trailing;
    let _: dot::center = horizontal.center;
    let _: dot::r#move = PointerStyle::dot_values().r#move;
    let all = dot::Values::default();
    let _: dot::accent = all.accent;
    let _: dot::infinity = all.infinity;
    let _: dot::bgra = all.bgra;
    assert_eq!(
        HorizontalAlignment::from(horizontal.leading),
        HorizontalAlignment::Leading
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Density {
    Compact,
    Comfortable,
}

struct DensityValues {
    compact: Density,
    comfortable: Density,
}

impl Default for DensityValues {
    fn default() -> Self {
        Self {
            compact: Density::Compact,
            comfortable: Density::Comfortable,
        }
    }
}

impl DotValues for Density {
    type Values = DensityValues;
}

use elegance::HorizontalAlignment as ImportedAlignment;
type AlignmentAlias = ImportedAlignment;

#[component]
fn UserDefinedProps(density: Density, alignment: AlignmentAlias) -> impl IntoView {
    let _ = (density, alignment);
    EmptyView()
}

struct GenericBuilder<T>(std::marker::PhantomData<T>);

#[elg_builder]
impl<T> GenericBuilder<T> {
    pub fn value(self, value: impl Into<T>) -> T {
        value.into()
    }
}

#[test]
fn user_namespaces_imports_and_aliases_are_resolved_by_type() {
    let component = elg! { UserDefinedProps(density: .comfortable, alignment: .leading) };
    assert_eq!(component.density, Density::Comfortable);
    assert_eq!(component.alignment, HorizontalAlignment::Leading);

    let builder = GenericBuilder::<Density>(std::marker::PhantomData);
    assert_eq!(elg! { builder.value(.compact) }, Density::Compact);

    let typed = DotTarget::<Density>::new();
    let namespace: DensityValues = (&typed).resolve_dot_values();
    assert_eq!(namespace.comfortable, Density::Comfortable);
    let alias = DotTarget::<AlignmentAlias>::new();
    let _: dot::leading = (&alias).resolve_dot_values().leading;
}

#[derive(Default)]
struct CustomOptions {
    gutter: usize,
}

impl CustomOptions {
    fn gutter(mut self, value: usize) -> Self {
        self.gutter = value;
        self
    }
}

struct CustomModifier;

#[elg_builder]
impl CustomModifier {
    pub fn identity(self, value: usize) -> usize {
        value
    }

    pub fn configure_with(
        self,
        options: impl FnOnce(CustomOptions) -> CustomOptions,
    ) -> CustomOptions {
        options(CustomOptions::default())
    }
}

#[test]
fn custom_options_are_discovered_from_the_with_signature() {
    let modifier = CustomModifier;
    let options = elg! { modifier.configure(gutter: 24) };
    assert_eq!(options.gutter, 24);

    let modifier = CustomModifier;
    let width = 40;
    assert_eq!(elg! { modifier.identity(width) }, width);
}

#[test]
fn unknown_and_same_named_types_need_no_dot_values_bound() {
    struct Color;
    struct Axis;

    let target = DotTarget::<Color>::new();
    let _: dot::Values = (&target).resolve_dot_values();
    let target = DotTarget::<Axis>::new();
    let _: dot::Values = (&target).resolve_dot_values();
    let target = DotTarget::<std::time::Duration>::new();
    let _: dot::Values = (&target).resolve_dot_values();
}
