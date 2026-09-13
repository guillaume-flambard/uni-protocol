use mini_app::bump;

#[test]
fn bump_ten() {
    assert_eq!(bump(100, 10), 110);
}
