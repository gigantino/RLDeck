use super::*;
use crate::testing::scratch as tmp;

#[test]
fn a_copy_reports_every_byte_it_writes() {
    let dir = tmp("copy");
    let from = dir.join("in.upk");
    let to = dir.join("out.upk");

    fs::write(&from, vec![7u8; CHUNK + 12_345]).unwrap();

    let progress = Progress::default();
    progress.start(CHUNK as u64 + 12_345);
    copy(&from, &to, &progress).unwrap();

    assert_eq!(progress.done(), CHUNK as u64 + 12_345);
    assert_eq!(progress.fraction(), Some(1.0));
    assert_eq!(fs::read(&to).unwrap(), fs::read(&from).unwrap());
}

#[test]
fn an_unknown_size_has_no_fraction_rather_than_a_wrong_one() {
    let progress = Progress::default();
    assert_eq!(progress.fraction(), None, "a bar at zero would be a claim we can't make");

    progress.add(500);
    assert_eq!(progress.fraction(), None);
}

#[test]
fn overshooting_the_estimate_still_reads_as_finished() {
    let progress = Progress::default();
    progress.start(100);
    progress.add(250);
    assert_eq!(progress.fraction(), Some(1.0));
}

#[test]
fn starting_again_forgets_the_last_operation() {
    let progress = Progress::default();
    progress.start(100);
    progress.add(100);

    progress.start(400);
    assert_eq!(progress.done(), 0);
    assert_eq!(progress.fraction(), Some(0.0));
}
