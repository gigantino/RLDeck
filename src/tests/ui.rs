use super::*;

#[test]
fn the_bar_never_asks_the_layout_for_a_zero_share() {
    for fraction in [None, Some(0.0), Some(0.5), Some(1.0), Some(-1.0), Some(2.0)] {
        let (filled, empty) = portions(fraction);

        assert_eq!(filled + empty, 1000, "{fraction:?} lost part of the track");
        assert!(filled <= 1000 && empty <= 1000, "{fraction:?} overflowed");
    }

    assert_eq!(portions(None), (0, 1000), "an unknown size draws an empty track");
    assert_eq!(portions(Some(1.0)), (1000, 0), "a finished copy fills it");
    assert_eq!(portions(Some(0.25)), (250, 750));
}

#[test]
fn labels_read_the_same_in_a_bar_and_in_a_sentence() {
    assert_eq!(capitalised("importing Cool Map"), "Importing Cool Map");
    assert_eq!(capitalised(""), "");
    assert_eq!(capitalised("\u{e9}lan"), "\u{c9}lan");
}

#[test]
fn sizes_are_reported_in_whole_megabytes() {
    assert_eq!(megabytes(0), "0 MB");
    assert_eq!(megabytes(4_120_000), "4 MB");
    assert_eq!(megabytes(999_999_999), "1000 MB");
}
