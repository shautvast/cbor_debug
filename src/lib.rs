use std::collections::HashMap;
use crate::MajorType::*;


pub fn decode(bytes: &[u8]) -> String {
    let mut output = Vec::new();
    let mut idx = 0;
    while idx < bytes.len() {
        output.push(decode_at(bytes, &mut idx));
        idx += 1;
    }
    format!("{:?}", output)
}

fn decode_at(bytes: &[u8], mut idx: &mut usize) -> MajorType {
    let major_type = (bytes[*idx] & 0b11100000) >> 5;

    match major_type {
        0 => get_int(&bytes, idx).map(|v| U(v)).unwrap_or(Invalid),
        1 => get_int(&bytes, idx).map(|v| N(-1 - (v as i128))).unwrap_or(Invalid),
        2 => {
            let len = get_int(&bytes, &mut idx).unwrap() as usize;
            let byte_string = bytes[*idx..*idx + len].to_vec();
            *idx += len;
            BStr(byte_string)
        }
        3 => {
            let len = get_int(&bytes, idx).unwrap() as usize;
            let utf = bytes[*idx..*idx + len].to_vec();
            *idx += len;
            Str(String::from_utf8(utf).unwrap())
        }
        4 => {
            let len = get_int(&bytes, idx).unwrap() as usize;
            let mut array: Vec<MajorType> = Vec::new();
            for _ in 0..len {
                array.push(decode_at(bytes, idx));
            }
            Arr(array)
        }
        5 => Map(HashMap::new()),
        6 => Tag,
        7 => {
            let additional = bytes[*idx] & 0b00011111;
            let out = match additional {
                20 => False,
                21 => True,
                22 => Null,
                23 => Undefined,
                25 => F16(get_f16(&bytes, idx)),
                26 => F32(get_f32(&bytes, idx)),
                27 => F64(get_f64(&bytes, idx)),
                _ => Invalid
            };
            *idx += 1;
            out
        }
        _ => {
            Invalid
        }
    }
}

fn get_f16(bytes: &[u8], idx: &mut usize) -> f32 {
    let b1 = bytes[*idx + 1];
    let b2 = bytes[*idx + 2];
    *idx += 2;
    let sign = if (b1 & 0b10000000) == 0 { 1.0_f32 } else { -1.0_f32 };
    let exponent = ((b1 & 0b01111100) >> 2) as i32 - 15;
    let fraction = ((((b1 & 0b00000011) as u16) << 8) + b2 as u16) as f32;
    2.0_f32.powi(exponent) * (1.0_f32 + fraction / 1024_f32) * sign
}

fn get_f32(bytes: &[u8], idx: &mut usize) -> f32 {
    *idx += 4;
    f32::from_be_bytes(to_b4(&bytes[*idx - 3..=*idx]))
}

fn get_f64(bytes: &[u8], idx: &mut usize) -> f64 {
    *idx += 8;
    f64::from_be_bytes(to_b8(&bytes[*idx - 7..=*idx]))
}

fn get_int(bytes: &[u8], i: &mut usize) -> Option<u64> {
    let additional = bytes[*i] & 0b00011111;
    if additional < 24 {
        *i += 1;
        Some(additional as u64)
    } else {
        if additional < 28 {
            let nbytes = 1 << (additional - 24);

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
    U(u64),
    N(i128),
    BStr(Vec<u8>),
    Str(String),
    Arr(Vec<MajorType>),
    Map(HashMap<String, MajorType>),
    Tag,
    False,
    True,
    Null,
    Undefined,
    F16(f32),
    F32(f32),
    F64(f64),
    Invalid,
}

fn to_b8(bytes: &[u8]) -> [u8; 8] {
    let mut out = [0_u8; 8];
    for (i, b) in bytes.iter().enumerate() {
        out[8 - bytes.len() + i] = *b;
    }
    out
}

fn to_b4(bytes: &[u8]) -> [u8; 4] {
    let mut out = [0_u8; 4];
    for (i, b) in bytes.iter().enumerate() {
        out[4 - bytes.len() + i] = *b;
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
    fn float16() {
        assert_eq!("[F16(1.0009766)]", decode(&[249, 60, 1]));
    }

    #[test]
    fn float32() {
        assert_eq!("[F32(1.0)]", decode(&to_vec(1.0_f32).unwrap()));
    }

    #[test]
    fn float64() {
        assert_eq!("[F64(10088000023.10022)]", decode(&to_vec(10088000023.10022_f64).unwrap()));
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
        assert_eq!(format!("[Arr([Null, Str(\"foobar\")])]"), decode(&to_vec(Simple { name: "foobar".into() }).unwrap()));
    }

    #[test]
    fn enum_1() {
        assert_eq!(format!("[Arr([U(1), Arr([Null, Str(\"foo\")])])]"), decode(
            &to_vec(
                Simple::Left("foo".into())
            ).unwrap()));
    }

    #[test]
    fn enum_vec() {
        assert_eq!(format!("[Arr([Arr([U(1), Arr([Null, Str(\"foo\")])]), Arr([U(2), Arr([Null, Str(\"bar\")])])])]"), decode(
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