// Counter: state and reactive arguments. Writing `count` updates exactly the views whose
// closures read it; nothing else is rebuilt. (A port of EleganceMain/examples/counter.)

use std::process::ExitCode;

use elegance::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let mut count = State::new(0);

    elg! {
        VStack(spacing: 16) {
            Text("{count}").font(Font::system(56.0, .bold))
            HStack {
                Button("−", move || count -= 1)
                Button("Reset", move || count.set(0))
                    .button_style(BorderedButtonStyle)
                    .disabled(move || count.get() == 0)
                Button("+", move || count += 1)
            }
            Text(move || if count.get() % 2 == 0 { "even" } else { "odd" })
                .foreground_style(.secondary)
        }
    }
}

fn main() -> Result<ExitCode, Error> {
    App::new().window(WindowDesc::new("Counter").size(360, 280), Counter).run()
}
