//! `elg!`, a SwiftUI-style DSL for Elegance views, and `#[component]`. See the `elegance`
//! crate's docs for the syntax.

mod component;
mod dot;
mod dsl;
mod methods;
mod preview;
mod rewrite;

use proc_macro::TokenStream;

/// Adds typed leading-dot completion helpers to public methods. Bare `#[elg_builder]`
/// does not register a constructor; types without `DotValues` retain global marker lookup.
///
/// `#[elg_builder(constructor)]` also implements `ElgNew<()>` for an inherent impl with
/// a safe, synchronous, nongeneric `fn new() -> Self` taking no arguments. This enables
/// `elg!(Name())` and named properties, but does not implement `Component` or `IntoView`.
/// Do not use it if `ElgNew<()>` is already implemented.
#[proc_macro_attribute]
pub fn elg_builder(attr: TokenStream, item: TokenStream) -> TokenStream {
    methods::expand(attr.into(), item.into()).into()
}

/// Defines dot markers, conversions, and their per-target completion namespaces.
#[proc_macro]
pub fn dots(input: TokenStream) -> TokenStream {
    dot::expand(input.into()).into()
}

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
/// Every prop gets a leading-dot type witness. Imported names, aliases, and custom
/// `elegance::dot::DotValues` implementations support typed completion; other prop
/// types retain global marker lookup without an additional trait bound.
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    component::expand(attr.into(), item.into()).into()
}

/// Registers a preview for the preview host (`cargo elegance preview`), named by the argument or
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
