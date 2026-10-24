// Todos: a list in state. TextField and Binding, ForEach keyed by id, `if`, Computed, and
// animated insertions and removals. (A port of EleganceMain/examples/todos.)

use std::process::ExitCode;

use elegance::prelude::*;

#[derive(Clone, PartialEq)]
struct Todo {
    id: i64,
    title: String,
    done: bool,
}

#[component]
fn TodoRow(id: i64, title: String, todos: State<Vec<Todo>>) -> impl IntoView {
    // A Binding reads and writes one field of one item in the list.
    let done = Binding::new(
        move || todos.with(|list| list.iter().any(|t| t.id == id && t.done)),
        move |value| {
            todos.update(|list| {
                if let Some(todo) = list.iter_mut().find(|t| t.id == id) {
                    todo.done = value;
                }
            })
        },
    );

    let remove = move || with_animation(Animation::smooth(0.4), || todos.update(|list| list.retain(|t| t.id != id)));

    elg! {
        HStack {
            Toggle(title, done).toggle_style(CheckboxToggleStyle)
            Spacer()
            Button(Image("trash"), remove)
                .button_style(PlainButtonStyle)
                .foreground_style(.danger)
                .accessibility_label("Delete")
        }
    }
}

#[component]
fn TodoApp() -> impl IntoView {
    let todos = State::new(vec![
        Todo { id: 1, title: "Read the docs".into(), done: false },
        Todo { id: 2, title: "Try the examples".into(), done: false },
        Todo { id: 3, title: "Build something".into(), done: false },
    ]);
    let draft = State::new(String::new());
    let mut next_id = State::new(4_i64);
    let remaining = computed(move || todos.with(|list| list.iter().filter(|t| !t.done).count()));

    let add = move || {
        if draft.with(String::is_empty) {
            return;
        }
        with_animation(Animation::smooth(0.4), || {
            todos.update(|list| list.push(Todo { id: next_id.get(), title: draft.get(), done: false }));
        });
        next_id += 1;
        draft.set(String::new());
    };

    elg! {
        VStack(alignment: .leading, spacing: 12) {
            Text("Todos").font(.large_title)
            HStack {
                TextField("What needs doing?", $draft, on_submit: add)
                Button("Add", add).disabled(move || draft.with(String::is_empty))
            }
            if todos.with(Vec::is_empty) {
                Text("Nothing left to do.").foreground_style(.secondary)
            } else {
                List {
                    ForEach(todos, id: |t| t.id) { todo in
                        TodoRow(id: todo.id, title: todo.title.clone(), todos: todos)
                    }
                }
            }
            Text("{remaining} left")
                .font(.caption)
                .foreground_style(.secondary)
        }
        .padding(24)
        .frame(max_width: .infinity, max_height: .infinity, alignment: .top_leading)
    }
}

fn main() -> Result<ExitCode, Error> {
    App::new().window(WindowDesc::new("Todos").size(480, 560).min_size(360.0, 320.0), TodoApp).run()
}
