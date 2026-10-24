//! `elg!`, a SwiftUI-style DSL for Elegence views, and `#[component]`. See the `elegence`
//! crate's docs for the syntax.

mod component;
mod dsl;
mod preview;
mod rewrite;

use proc_macro::TokenStream;

/// Builds a view from a SwiftUI-style description:
///
/// ```ignore
/// elg! {
///     VStack(alignment: .leading, spacing: 12) {
///         Text("Todos").font(.large_title)
///         HStack {
///             TextField("What needs doing?", $draft, on_submit: add)
///             Button("Add", add).disabled(move || draft.with(String::is_empty))
///         }
///         if todos.with(Vec::is_empty) {
///             Text("Nothing left to do.").foreground_style(.secondary)
///         } else {
///             List { ForEach(todos, id: |t| t.id) { todo in TodoRow(todo: todo.clone(), todos: todos) } }
///         }
///     }
///     .padding(24)
/// }
/// ```
#[proc_macro]
pub fn elg(input: TokenStream) -> TokenStream {
    dsl::expand(input.into()).into()
}

/// Makes a component from a function: its parameters become props, set by name in `elg!`
/// (`TodoRow(id: 1, title: t)`). Props must be `Clone + 'static`; mark optional ones
/// `#[prop(default)]` or `#[prop(default = expr)]` (an `Option<T>` prop defaults to `None`).
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    component::expand(attr.into(), item.into()).into()
}

/// Registers a preview for the preview host (`cargo elegence preview`), named by the argument or
/// the function; `#[preview(root)]` marks the app's root view instead (`--root`). The function
/// takes nothing and returns a view. Only preview builds register previews.
///
/// ```ignore
/// #[preview("Counter at 5")]
/// fn counter_five() -> impl IntoView {
///     elg! { Counter() }
/// }
/// ```
#[proc_macro_attribute]
pub fn preview(attr: TokenStream, item: TokenStream) -> TokenStream {
    preview::expand(attr.into(), item.into()).into()
}
