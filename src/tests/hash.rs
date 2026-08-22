use super::*;

#[test]
fn every_byte_becomes_two_lowercase_digits() {
    assert_eq!(hex(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    assert_eq!(hex(&[]), "");
}

#[test]
fn a_file_and_its_bytes_hash_the_same() {
    let dir = crate::testing::scratch("hash");
    let path = dir.join("payload.bin");
    let payload = vec![0x5Au8; CHUNK * 2 + 7];
    fs::write(&path, &payload).unwrap();

    assert_eq!(of_file(&path).unwrap(), of_bytes(&payload));
}

#[test]
fn a_short_digest_is_a_prefix_of_the_long_one() {
    let full = of_bytes(b"/games/rocketleague");
    assert_eq!(short(b"/games/rocketleague", 6), full[..12]);
    assert_ne!(short(b"/games/rocketleague", 6), short(b"/other", 6));
}
