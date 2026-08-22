use super::*;

#[test]
fn the_same_label_twice_is_two_different_folders() {
    let one = scratch("multi");
    let two = scratch("multi");

    assert_ne!(one, two);
    assert!(one.is_dir() && two.is_dir());
}
