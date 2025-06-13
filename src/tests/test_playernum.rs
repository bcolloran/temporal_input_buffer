use crate::util_types::PlayerNum;
use std::convert::TryFrom;

#[test]
fn test_from_u8() {
    let num: PlayerNum = 5u8.into();
    assert_eq!(num, PlayerNum(5));
}

#[test]
fn test_try_from_u32() {
    let num = PlayerNum::try_from(42u32).unwrap();
    assert_eq!(num, PlayerNum(42));
}

#[test]
fn test_try_from_u32_out_of_range() {
    assert!(PlayerNum::try_from(300u32).is_err());
}

#[test]
fn test_try_from_usize() {
    let num = PlayerNum::try_from(100usize).unwrap();
    assert_eq!(num, PlayerNum(100));
}

#[test]
fn test_try_from_usize_out_of_range() {
    assert!(PlayerNum::try_from(300usize).is_err());
}

#[test]
fn test_into_values() {
    let num = PlayerNum(7);
    let val_u8: u8 = num.into();
    assert_eq!(val_u8, 7);

    let num = PlayerNum(8);
    let val_u32: u32 = num.into();
    assert_eq!(val_u32, 8);

    let num = PlayerNum(9);
    let val_string: String = num.into();
    assert_eq!(val_string, "9");
}
