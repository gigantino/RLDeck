use super::*;
use crate::testing::scratch as tmp;

#[test]
fn a_completed_write_leaves_only_the_destination() {
    let dir = tmp("ok");
    let dest = dir.join("config.json");

    write(&dest, |part| fs::write(part, b"payload")).unwrap();

    assert_eq!(fs::read(&dest).unwrap(), b"payload");
    assert!(!part_of(&dest).exists(), "the staging file must not survive");
}

#[test]
fn a_failed_write_leaves_neither_the_destination_nor_the_staging_file() {
    let dir = tmp("fail");
    let dest = dir.join("config.json");

    let err = write(&dest, |_| Err(io::Error::other("disk full"))).unwrap_err();

    assert_eq!(err.to_string(), "disk full");
    assert!(!dest.exists());
    assert!(!part_of(&dest).exists());
}

#[test]
fn a_failed_write_does_not_destroy_what_was_already_there() {
    let dir = tmp("keep");
    let dest = dir.join("config.json");
    fs::write(&dest, b"original").unwrap();

    let _ = write(&dest, |_| Err(io::Error::other("disk full")));

    assert_eq!(fs::read(&dest).unwrap(), b"original");
}

#[test]
fn destinations_sharing_a_stem_do_not_share_a_staging_file() {
    let dir = tmp("stems");
    assert_ne!(part_of(&dir.join("a.jpg")), part_of(&dir.join("a.png")));
}
