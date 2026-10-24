//! What `elg!` expands to. An element `Name(a, b, key: v) { children }` becomes
//!
//! ```ignore
//! Name::elg_new((a, b)).key(v).elg_children(vec![...])
//! ```
//!
//! so any type can be an element: implement [`ElgNew`] for its positional arguments (one impl per
//! overload, by tuple), builder methods for its named arguments, and [`ElgChildren`] if it takes
//! a trailing block. A block that starts with `item in` calls an inherent
//! `elg_content(move |item| ...)` method instead (ForEach, AsyncImage).

use std::rc::Rc;

use crate::prop::{IntoProp, Prop};
use crate::text::Text;
use crate::view::{IntoView, View};
use crate::views::{IntoLabel, LabelArg};

/// Makes an element from its positional arguments, as a tuple.
pub trait ElgNew<Args> {
    type Out;
    fn elg_new(args: Args) -> Self::Out;
}

/// Takes the views of an element's trailing block.
pub trait ElgChildren {
    type Out;
    fn elg_children(self, children: Vec<View>) -> Self::Out;
}

/// `"Count: {count}"` in `elg!`: text that formats again when state it reads changes. It's a
/// reactive string where Elegance takes one (`Text`, accessibility labels), a reactive Text as a
/// control's label, and formatted once where a plain `String` is needed.
pub struct Interpolated<F>(pub F);

impl<F: Fn() -> String + 'static> IntoProp<String> for Interpolated<F> {
    fn into_prop(self) -> Prop<String> {
        Prop::Fn(Rc::new(self.0))
    }
}

impl<F: Fn() -> String + 'static> IntoLabel for Interpolated<F> {
    fn into_label(self) -> LabelArg {
        LabelArg::View(Text::new(self).into_view())
    }
}

impl<F: Fn() -> String> From<Interpolated<F>> for String {
    fn from(text: Interpolated<F>) -> String {
        (text.0)()
    }
}

/// A required `#[component]` prop that hasn't been set.
#[derive(Clone, Copy, Debug)]
pub struct Missing;

/// `if cond { a } else { b }` in `elg!`: shows `then_view` while `condition` holds.
pub fn if_view(condition: impl IntoProp<bool>, then_view: impl IntoView, else_view: impl IntoView) -> View {
    crate::views::make_if(condition.into_prop(), then_view.into_view(), else_view.into_view())
}

/// `match value { pattern => view, ... }` in `elg!`: shows the view for the current value. When
/// the value changes, the old view (and state created in it) is removed and `arms` builds the new
/// one, as a ForEach of one item does.
pub fn match_view<T, V>(value: impl Fn() -> T + 'static, arms: impl Fn(&T) -> V + 'static) -> View
where
    T: Clone + PartialEq + 'static,
    V: IntoView,
{
    crate::views::for_each_view(
        Rc::new(move || vec![value()]),
        Rc::new(|_: &T| crate::views::ItemKey::Int(0)),
        Rc::new(move |item: &T| arms(item).into_view()),
    )
}

/// One view from a block's children: the view itself, or a VStack of several.
pub(crate) fn single(children: Vec<View>, what: &str) -> View {
    let mut children = children;
    match children.len() {
        0 => View::empty(),
        1 => children.pop().unwrap(),
        _ => {
            let _ = what;
            crate::views::VStack::new().children(children).into_view()
        }
    }
}

/// The view of a block that must hold exactly one (a ForEach for List, ...).
#[track_caller]
pub(crate) fn exactly_one(children: Vec<View>, what: &str) -> View {
    let mut children = children;
    if children.len() != 1 {
        panic!("elegance: {what} takes exactly one view in its block (got {})", children.len());
    }
    children.pop().unwrap()
}
