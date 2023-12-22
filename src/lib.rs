use std::collections::HashMap;
use crate::MajorType::*;


pub fn decode(bytes: &[u8]) -> String {
    format!("{:?}", decode_at(bytes, 0))
}

fn decode_at(bytes: &[u8], mut idx: usize) -> Vec<MajorType> {
    let mut output = Vec::new();

    while idx < bytes.len() {
        output.push(decode_one_at(bytes, &mut idx));
        idx += 1;
    }
    output
}

fn decode_one_at(bytes: &[u8], mut idx: &mut usize) -> MajorType {
    let major_type = (bytes[*idx] & 0b11100000) >> 5;

    match major_type {
        0 => get_intval(&bytes, idx).map(|v| U(v)).unwrap_or(Invalid),
        1 => get_intval(&bytes, idx).map(|v| N(-1 - (v as i128))).unwrap_or(Invalid),
        2 => {
            let len = get_intval(&bytes, &mut idx).unwrap() as usize;
            let byte_string = bytes[*idx..*idx + len].to_vec();
            *idx += len;
            BStr(byte_string)
        }
        3 => {
            let len = get_intval(&bytes, idx).unwrap() as usize;
            let utf = bytes[*idx..*idx + len].to_vec();
            *idx += len;
            Str(String::from_utf8(utf).unwrap())
        }
        4 => {
            let len = get_intval(&bytes, idx).unwrap() as usize;
            let mut array: Vec<MajorType> = Vec::new();
            for _ in 0..len {
                array.push(decode_one_at(bytes, idx));
            }
            Arr(array)
        }
        5 => Map(HashMap::new()),
        6 => Tag,
        7 => {
            *idx += 1;
            Div
        }
        _ => {
            Invalid
        }
    }
}

fn get_intval(bytes: &[u8], i: &mut usize) -> Option<u64> {
    let next = bytes[*i] & 0b00011111;
    if next < 24 {
        *i += 1;
        Some(next as u64)
    } else {
        if next < 28 {
            let nbytes = 1 << (next - 24);

            let int_val = u64::from_be_bytes(to_b8(&bytes[*i + 1..=*i + nbytes]));
            *i += nbytes + 1;
            Some(int_val)
        } else {
            None
        }
    }
}

#[derive(Debug)]
#[repr(u8)]
enum MajorType {
    U(u64) = 0,
    N(i128) = 1,
    BStr(Vec<u8>) = 2,
    Str(String) = 3,
    Arr(Vec<MajorType>) = 4,
    Map(HashMap<String, MajorType>) = 5,
    Tag = 6,
    Div = 7,
    Invalid,
}

fn to_b8(bytes: &[u8]) -> [u8; 8] {
    let mut out = [0_u8; 8];
    for (i, b) in bytes.iter().enumerate() {
        out[8 - bytes.len() + i] = *b;
    }
    out
}

#[cfg(test)]
mod test {
    use minicbor::{Decode, Encode, to_vec};
    use super::*;

    #[test]
    fn int_0() {
        assert_eq!("[U(0)]", decode(&to_vec(0).unwrap()));
    }

    #[test]
    fn int_24() {
        assert_eq!("[U(24)]", decode(&to_vec(24).unwrap()));
    }

    #[test]
    fn int_u64_max() {
        assert_eq!(format!("[U({})]", u64::MAX - 1), decode(&to_vec(u64::MAX - 1).unwrap()));
    }

    #[test]
    fn neg_int_23() {
        assert_eq!("[N(-23)]", decode(&to_vec(-23).unwrap()));
    }

    #[test]
    fn neg_int_i64_max() {
        // i64::MIN = -2^63
        //can't encode -2^64 in minicbor...? cbor spec allows it!
        assert_eq!(format!("[N({})]", i64::MIN), decode(&to_vec(i64::MIN).unwrap()));
    }

    #[test]
    fn bytestring() {
        assert_eq!(format!("[BStr([1, 2, 3, 4, 5])]"), decode(&[0b01000101, 1, 2, 3, 4, 5]));
    }

    #[test]
    fn string() {
        assert_eq!(format!("[Str(\"Hello World\")]"), decode(&to_vec("Hello World").unwrap()));
    }

    #[test]
    fn array() {
        assert_eq!(format!("[Arr([U(1), U(2), U(3), U(4), U(5)])]"), decode(&to_vec([1, 2, 3, 4, 5]).unwrap()));
    }

    #[test]
    fn struct_n0() {
        #[derive(Decode, Encode)]
        struct Simple {
            #[n(0)] name: String,
        }
        assert_eq!(format!("[Arr([Str(\"foobar\")])]"), decode(&to_vec(Simple { name: "foobar".into() }).unwrap()));
    }

    #[test]
    fn struct_n1() {
        #[derive(Decode, Encode)]
        struct Simple {
            #[n(1)] name: String,
        }
        assert_eq!(format!("[Arr([Div, Str(\"foobar\")])]"), decode(&to_vec(Simple { name: "foobar".into() }).unwrap()));
    }

    #[test]
    fn enum_1() {
        assert_eq!(format!("[Arr([U(1), Arr([Div, Str(\"foo\")])])]"), decode(
            &to_vec(
                Simple::Left("foo".into())
            ).unwrap()));
    }

    #[test]
    fn enum_vec() {
        assert_eq!(format!("[Arr([Arr([U(1), Arr([Div, Str(\"foo\")])]), Arr([U(2), Arr([Div, Str(\"bar\")])])])]"), decode(
            &to_vec(
                vec![Simple::Left("foo".into()),
                     Simple::Right("bar".into())],
            ).unwrap()));
    }

    #[derive(Decode, Encode)]
    enum Simple {
        #[n(1)] Left(#[n(1)] String),
        #[n(2)] Right(#[n(1)] String),
    }
}