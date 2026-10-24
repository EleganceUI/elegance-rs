// Effects: materials (background blur), blur, color adjustments, blend modes, shadows (layered,
// spread, inner), gradients, rotation and scale, and masks. (A port of
// ElegenceMain/examples/effects.)

use std::process::ExitCode;

use elegence::prelude::*;

// Something colorful to put effects over.
fn backdrop() -> impl IntoView {
    let colors = [0xFF5F6D, 0xFFC371, 0x47CF73, 0x3EA6FF, 0xA36BFF, 0xFF5F6D].map(Color::hex);
    elg! {
        ZStack {
            Rectangle().fill(AngularGradient::colors(colors))
            Circle().fill(Color::WHITE.opacity(0.85)).frame(70, 70).offset(-90, -30)
            Text("Elegence").font(Font::system(44.0, .bold)).foreground_style(.black).offset(30, 30)
        }
    }
}

fn caption(content: impl IntoView, label: &str) -> impl IntoView {
    let content = content.into_view();
    elg! {
        VStack(spacing: 6) {
            { content }
            Text(label).font(.caption).foreground_style(.secondary)
        }
    }
}

fn section(title: &str, content: impl IntoView) -> impl IntoView {
    let content = content.into_view();
    elg! {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.headline)
            { content }
        }
    }
}

fn materials() -> impl IntoView {
    let items = [
        (Material::ULTRA_THIN, "ultra thin"),
        (Material::THIN, "thin"),
        (Material::REGULAR, "regular"),
        (Material::THICK, "thick"),
        (Material::ULTRA_THICK, "ultra thick"),
    ];
    section(
        "Materials (background blur)",
        elg! {
            ZStack {
                { backdrop() }
                HStack(spacing: 8) {
                    ForEach(items.to_vec(), id: |m| m.1) { m in
                        Text(m.1)
                            .font(.caption)
                            .padding(10)
                            .frame(max_width: .infinity, max_height: .infinity)
                            .background_in(m.0, RoundedRectangle(12))
                    }
                }
                .padding(14)
            }
            .frame(height: 140)
            .corner_radius(16)
        },
    )
}

fn blur() -> impl IntoView {
    let radius = State::new(6.0f32);
    section(
        "Blur",
        elg! {
            VStack(spacing: 8) {
                { backdrop() }.frame(height: 110).corner_radius(16).blur(radius)
                HStack {
                    Text("Radius")
                    Slider($radius, min: 0, max: 30)
                }
            }
        },
    )
}

fn tile() -> impl IntoView {
    elg! {
        ZStack {
            Circle().fill(LinearGradient::colors([Color::hex(0xFF8A5B), Color::hex(0x7B5BFF)]))
            Image("sparkle").font(Font::system(26.0, .regular)).foreground_style(.white)
        }
        .frame(56, 56)
    }
}

fn color_adjustments() -> impl IntoView {
    section(
        "Color adjustments",
        elg! {
            HStack(spacing: 12) {
                { caption(tile(), "original") }
                { caption(tile().grayscale(1), "grayscale") }
                { caption(tile().sepia(1), "sepia") }
                { caption(tile().hue_rotation(120), "hue 120°") }
                { caption(tile().saturation(2.2), "saturation") }
                { caption(tile().contrast(1.8), "contrast") }
                { caption(tile().brightness(0.25), "brightness") }
                { caption(tile().color_invert(), "invert") }
                { caption(tile().color_multiply(Color::hex(0x47CF73)), "multiply") }
            }
        },
    )
}

fn blend_sample(mode: BlendMode, name: &str) -> impl IntoView {
    let background = LinearGradient::colors([Color::hex(0x3EA6FF), Color::hex(0xFFC371)])
        .from_to(UnitPoint::LEADING, UnitPoint::TRAILING);
    caption(
        elg! {
            ZStack {
                Rectangle().fill(background)
                Circle().fill(Color::hex(0xFF5F6D)).frame(38, 38).blend_mode(mode)
            }
            .frame(64, 52)
            .corner_radius(10)
        },
        name,
    )
}

fn blend_modes() -> impl IntoView {
    use BlendMode::*;
    section(
        "Blend modes",
        elg! {
            VStack(spacing: 10) {
                HStack(spacing: 10) {
                    { blend_sample(Normal, "normal") }
                    { blend_sample(Multiply, "multiply") }
                    { blend_sample(Screen, "screen") }
                    { blend_sample(Overlay, "overlay") }
                    { blend_sample(Darken, "darken") }
                    { blend_sample(Lighten, "lighten") }
                }
                HStack(spacing: 10) {
                    { blend_sample(ColorDodge, "dodge") }
                    { blend_sample(ColorBurn, "burn") }
                    { blend_sample(SoftLight, "soft light") }
                    { blend_sample(Difference, "difference") }
                    { blend_sample(Hue, "hue") }
                    { blend_sample(Luminosity, "luminosity") }
                }
            }
        },
    )
}

fn shadows() -> impl IntoView {
    let card = || elg! { RoundedRectangle(14).fill(.white).frame(84, 64) };
    section(
        "Shadows",
        elg! {
            HStack(spacing: 22) {
                {
                    caption(
                        card()
                            .shadow_with(|s| s.color(Color::BLACK.opacity(0.18)).radius(2).y(1))
                            .shadow_with(|s| s.color(Color::BLACK.opacity(0.14)).radius(24).y(12)),
                        "layered",
                    )
                }
                { caption(card().shadow_with(|s| s.color(Color::BLACK.opacity(0.3)).radius(12).y(14).spread(-6)), "negative spread") }
                { caption(card().shadow_with(|s| s.color(Color::hex(0x3EA6FF).opacity(0.9)).radius(18)), "glow") }
                { caption(RoundedRectangle(14).fill(Color::hex(0xE9ECF2)).frame(84, 64).inner_shadow_with(|s| s), "inner shadow") }
                {
                    caption(
                        Text("Pressed")
                            .font(Font::system(28.0, .bold))
                            .foreground_style(Color::hex(0xD6DAE3))
                            .inner_shadow_with(|s| s.color(Color::BLACK.opacity(0.5)).radius(3).x(1).y(2)),
                        "inner shadow on text",
                    )
                }
            }
            .padding(18)
            .background_in(Color::hex(0xF4F5F8), RoundedRectangle(16))
        },
    )
}

fn gradients() -> impl IntoView {
    let rainbow = [0xFF0000, 0xFFFF00, 0x00FF00, 0x00FFFF, 0x0000FF, 0xFF00FF, 0xFF0000].map(Color::hex);
    section(
        "Gradients",
        elg! {
            HStack(spacing: 12) {
                { caption(Circle().fill(AngularGradient::colors(rainbow)).frame(64, 64), "angular") }
                {
                    caption(
                        Capsule().fill(EllipticalGradient::colors([Color::hex(0xFFE29F), Color::hex(0xFF719A)])).frame(110, 64),
                        "elliptical",
                    )
                }
                {
                    caption(
                        RoundedRectangle(14)
                            .fill(AngularGradient::colors([Color::hex(0x7B5BFF), Color::hex(0x3EA6FF)]).angles(-90.0, 90.0))
                            .frame(64, 64),
                        "angular, half turn",
                    )
                }
            }
        },
    )
}

fn transforms_and_masks() -> impl IntoView {
    let turned = State::new(false);
    let fade = LinearGradient::colors([Color::BLACK, Color::CLEAR]).from_to(UnitPoint::LEADING, UnitPoint::TRAILING);
    section(
        "Rotation, scale, masks",
        elg! {
            HStack(spacing: 22) {
                {
                    caption(
                        RoundedRectangle(12)
                            .fill(LinearGradient::colors([Color::hex(0x47CF73), Color::hex(0x3EA6FF)]))
                            .overlay(Text("Tap").foreground_style(.white))
                            .frame(64, 64)
                            .rotation_effect(move || if turned.get() { 135.0 } else { 15.0 })
                            .on_tap_gesture(move || with_animation(Animation::spring(0.5, 0.825), move || turned.set(!turned.get()))),
                        "rotation_effect",
                    )
                }
                { caption(Text("Wide").font(Font::system(28.0, .bold)).scale_effect_xy(1.6, 0.8), "scale_effect") }
                { caption(backdrop().frame(150, 64).clipped().mask(fade), "gradient mask") }
                { caption(backdrop().frame(64, 64).clipped().mask_view(Image("heart").font(Font::system(60.0, .regular))), "icon mask") }
                { caption(backdrop().frame(120, 64).clipped().compositing_group().opacity(0.4), "group opacity") }
            }
        },
    )
}

#[component]
fn Effects() -> impl IntoView {
    elg! {
        ScrollView {
            HStack(alignment: .top, spacing: 32) {
                VStack(alignment: .leading, spacing: 26) {
                    { materials() }
                    { blur() }
                    { gradients() }
                }
                VStack(alignment: .leading, spacing: 26) {
                    { color_adjustments() }
                    { blend_modes() }
                    { shadows() }
                    { transforms_and_masks() }
                }
            }
            .padding(24)
        }
    }
}

fn main() -> Result<ExitCode, Error> {
    App::new().window(WindowDesc::new("Effects").size(1240, 900), Effects).run()
}
