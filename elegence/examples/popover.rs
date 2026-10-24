// Popover: panels shown beside a view while a binding is true. Clicking outside or Esc closes
// them and sets the binding back. (A port of ElegenceMain/examples/popover.)

use std::process::ExitCode;

use elegence::prelude::*;

#[component]
fn Editor(name: State<String>, shared: State<bool>, editing: State<bool>) -> impl IntoView {
    elg! {
        VStack(alignment: .leading, spacing: 10) {
            Text("Rename").font(.headline)
            TextField("Name", $name)
            Toggle("Shared with the team", $shared)
            HStack {
                Spacer()
                Button("Done", move || editing.set(false))
            }
        }
        .frame(width: 240)
    }
}

#[component]
fn PopoverApp() -> impl IntoView {
    let editing = State::new(false);
    let showing_info = State::new(false);
    let name = State::new("Untitled".to_string());
    let shared = State::new(false);

    elg! {
        VStack(spacing: 16) {
            Text(move || format!("{}{}", name.get(), if shared.get() { " (shared)" } else { "" }))
                .font(.title)
            HStack(spacing: 12) {
                // The content closure builds the panel each time it opens.
                Button("Edit…", move || editing.set(true))
                    .popover($editing, move || elg! { Editor(name: name, shared: shared, editing: editing) })
                Button(Image("info"), move || showing_info.update(|s| *s = !*s))
                    .button_style(PlainButtonStyle)
                    .accessibility_label("About")
                    .popover($showing_info, || elg! {
                        Text("Popovers close when you click elsewhere or press Esc.").frame(width: 200)
                    }, opens: .trailing)
            }
            Text(move || format!("editing: {}", if editing.get() { "yes" } else { "no" }))
                .font(.caption)
                .foreground_style(.secondary)
        }
        .padding(24)
        .frame(max_width: .infinity, max_height: .infinity)
    }
}

fn main() -> Result<ExitCode, Error> {
    App::new().window(WindowDesc::new("Popover").size(480, 360), PopoverApp).run()
}
