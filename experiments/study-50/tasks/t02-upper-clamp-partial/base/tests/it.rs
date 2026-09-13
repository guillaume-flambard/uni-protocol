use mini_app::*;

#[test]
fn add_works() {
    assert_eq!(add(2, 3), 5);
}

#[test]
fn clamp_lower_works() {
    assert_eq!(clamp(1, 5, 10), 5);
}
