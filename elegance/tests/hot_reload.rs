// layout_hash! must match Elegance's layout_hash<T...>() (hot_reload.h): FNV-1a of each type's
// size and alignment, as 8 little-endian bytes each.

#[test]
fn layout_hash_matches_elegance() {
    assert_eq!(elegance::layout_hash!(), 14695981039346656037);
    assert_eq!(elegance::layout_hash!(u32, u64), 1285882993202171429);
    assert_eq!(elegance::hot_reload::layout_hash(&[(4, 4), (8, 8)]), 1285882993202171429);
    const HASH: u64 = elegance::layout_hash!(u32, u64);
    assert_ne!(HASH, elegance::layout_hash!(u64, u32));
}

// The attribute forms parse (previews register only in preview builds).
#[elegance::preview(root, app_hash = elegance::layout_hash!(u32))]
fn app_root() -> impl elegance::IntoView {
    elegance::Text("root")
}

#[elegance::preview("Named")]
fn named() -> impl elegance::IntoView {
    elegance::Text("named")
}

#[test]
fn preview_attributes_keep_their_functions() {
    let _ = app_root;
    let _ = named;
}
