use mini_app::first_word;

#[test]
fn normal_sentence() {
    assert_eq!(first_word("hello world"), "hello");
}
