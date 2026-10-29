//! Minimal NBT (Named Binary Tag) codec.
//!
//! Bedrock Edition (including old Pocket Edition) stores NBT little-endian;
//! Java Edition stores it big-endian. The tag model is identical, so one
//! [`Tag`] type serves both and the byte order is chosen per call.

use std::fmt;

/// Byte order of an NBT stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    /// Bedrock / Pocket Edition.
    Little,
    /// Java Edition.
    Big,
}

/// A single NBT value. Compound entries keep their on-disk order.
#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    /// Element type id plus elements. The id is kept so empty lists round-trip.
    List(u8, Vec<Tag>),
    Compound(Vec<(String, Tag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    /// The NBT type id of this tag.
    pub fn id(&self) -> u8 {
        match self {
            Tag::Byte(_) => 1,
            Tag::Short(_) => 2,
            Tag::Int(_) => 3,
            Tag::Long(_) => 4,
            Tag::Float(_) => 5,
            Tag::Double(_) => 6,
            Tag::ByteArray(_) => 7,
            Tag::String(_) => 8,
            Tag::List(..) => 9,
            Tag::Compound(_) => 10,
            Tag::IntArray(_) => 11,
            Tag::LongArray(_) => 12,
        }
    }

    /// Looks up a compound entry by name.
    pub fn get(&self, name: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(entries) => entries.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Numeric value of any integer or float tag, widened to `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        Some(match *self {
            Tag::Byte(v) => v as i64,
            Tag::Short(v) => v as i64,
            Tag::Int(v) => v as i64,
            Tag::Long(v) => v,
            Tag::Float(v) => v as i64,
            Tag::Double(v) => v as i64,
            _ => return None,
        })
    }

    /// Numeric value of any integer or float tag, as `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        Some(match *self {
            Tag::Float(v) => v as f64,
            Tag::Double(v) => v,
            _ => self.as_i64()? as f64,
        })
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Tag::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Tag]> {
        match self {
            Tag::List(_, items) => Some(items),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nbt: {}", self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
    endian: Endian,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.buf.len());
        let end = end.ok_or_else(|| Error(format!("unexpected end of data at byte {}", self.pos)))?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into().unwrap())
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
}

macro_rules! read_num {
    ($name:ident, $t:ty) => {
        fn $name(&mut self) -> Result<$t> {
            let b = self.array()?;
            Ok(match self.endian {
                Endian::Little => <$t>::from_le_bytes(b),
                Endian::Big => <$t>::from_be_bytes(b),
            })
        }
    };
}

impl Reader<'_> {
    read_num!(i16, i16);
    read_num!(u16, u16);
    read_num!(i32, i32);
    read_num!(i64, i64);
    read_num!(f32, f32);
    read_num!(f64, f64);

    fn len(&mut self) -> Result<usize> {
        let n = self.i32()?;
        usize::try_from(n).map_err(|_| Error(format!("negative length {n}")))
    }

    fn string(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        // Java uses modified UTF-8; lossy decoding is fine for the names we use.
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }

    fn payload(&mut self, id: u8, depth: usize) -> Result<Tag> {
        if depth > 512 {
            return Err(Error("nesting too deep".into()));
        }
        Ok(match id {
            1 => Tag::Byte(self.u8()? as i8),
            2 => Tag::Short(self.i16()?),
            3 => Tag::Int(self.i32()?),
            4 => Tag::Long(self.i64()?),
            5 => Tag::Float(self.f32()?),
            6 => Tag::Double(self.f64()?),
            7 => {
                let n = self.len()?;
                Tag::ByteArray(self.take(n)?.iter().map(|&b| b as i8).collect())
            }
            8 => Tag::String(self.string()?),
            9 => {
                let elem = self.u8()?;
                let n = self.len()?;
                let mut items = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    items.push(self.payload(elem, depth + 1)?);
                }
                Tag::List(elem, items)
            }
            10 => {
                let mut entries = Vec::new();
                loop {
                    let id = self.u8()?;
                    if id == 0 {
                        break;
                    }
                    let name = self.string()?;
                    entries.push((name, self.payload(id, depth + 1)?));
                }
                Tag::Compound(entries)
            }
            11 => {
                let n = self.len()?;
                Tag::IntArray((0..n).map(|_| self.i32()).collect::<Result<_>>()?)
            }
            12 => {
                let n = self.len()?;
                Tag::LongArray((0..n).map(|_| self.i64()).collect::<Result<_>>()?)
            }
            _ => return Err(Error(format!("unknown tag id {id} at byte {}", self.pos))),
        })
    }
}

/// Reads one named root tag from the start of `buf`.
///
/// Returns the root's name, the tag, and the number of bytes consumed, so
/// callers can read several concatenated roots (as Bedrock does for block
/// entities and entities).
pub fn read(buf: &[u8], endian: Endian) -> Result<(String, Tag, usize)> {
    let mut r = Reader { buf, pos: 0, endian };
    let id = r.u8()?;
    if id == 0 {
        return Err(Error("root is an End tag".into()));
    }
    let name = r.string()?;
    let tag = r.payload(id, 0)?;
    Ok((name, tag, r.pos))
}

/// Reads every concatenated root tag in `buf`.
pub fn read_all(mut buf: &[u8], endian: Endian) -> Result<Vec<Tag>> {
    let mut out = Vec::new();
    while !buf.is_empty() {
        let (_, tag, used) = read(buf, endian)?;
        out.push(tag);
        buf = &buf[used..];
    }
    Ok(out)
}

struct Writer<'a> {
    out: &'a mut Vec<u8>,
    endian: Endian,
}

macro_rules! write_num {
    ($name:ident, $t:ty) => {
        fn $name(&mut self, v: $t) {
            match self.endian {
                Endian::Little => self.out.extend_from_slice(&v.to_le_bytes()),
                Endian::Big => self.out.extend_from_slice(&v.to_be_bytes()),
            }
        }
    };
}

impl Writer<'_> {
    write_num!(i16, i16);
    write_num!(u16, u16);
    write_num!(i32, i32);
    write_num!(i64, i64);
    write_num!(f32, f32);
    write_num!(f64, f64);

    fn len(&mut self, n: usize) {
        self.i32(i32::try_from(n).expect("NBT array too long"));
    }

    fn string(&mut self, s: &str) {
        let b = s.as_bytes();
        self.u16(u16::try_from(b.len()).expect("NBT string too long"));
        self.out.extend_from_slice(b);
    }

    fn payload(&mut self, tag: &Tag) {
        match tag {
            Tag::Byte(v) => self.out.push(*v as u8),
            Tag::Short(v) => self.i16(*v),
            Tag::Int(v) => self.i32(*v),
            Tag::Long(v) => self.i64(*v),
            Tag::Float(v) => self.f32(*v),
            Tag::Double(v) => self.f64(*v),
            Tag::ByteArray(v) => {
                self.len(v.len());
                self.out.extend(v.iter().map(|&b| b as u8));
            }
            Tag::String(s) => self.string(s),
            Tag::List(elem, items) => {
                // An empty list conventionally carries element type End.
                let elem = items.first().map_or(*elem, Tag::id);
                self.out.push(elem);
                self.len(items.len());
                for item in items {
                    debug_assert_eq!(item.id(), elem, "heterogeneous NBT list");
                    self.payload(item);
                }
            }
            Tag::Compound(entries) => {
                for (name, value) in entries {
                    self.out.push(value.id());
                    self.string(name);
                    self.payload(value);
                }
                self.out.push(0);
            }
            Tag::IntArray(v) => {
                self.len(v.len());
                v.iter().for_each(|&x| self.i32(x));
            }
            Tag::LongArray(v) => {
                self.len(v.len());
                v.iter().for_each(|&x| self.i64(x));
            }
        }
    }
}

/// Appends `tag` as a named root tag to `out`.
pub fn write(out: &mut Vec<u8>, name: &str, tag: &Tag, endian: Endian) {
    let mut w = Writer { out, endian };
    w.out.push(tag.id());
    w.string(name);
    w.payload(tag);
}

/// Builds a compound from `(name, tag)` pairs.
pub fn compound<I, S>(entries: I) -> Tag
where
    I: IntoIterator<Item = (S, Tag)>,
    S: Into<String>,
{
    Tag::Compound(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Tag {
        compound([
            ("b", Tag::Byte(-3)),
            ("s", Tag::Short(300)),
            ("i", Tag::Int(-70000)),
            ("l", Tag::Long(1 << 40)),
            ("f", Tag::Float(1.5)),
            ("d", Tag::Double(-2.25)),
            ("ba", Tag::ByteArray(vec![1, -1, 7])),
            ("str", Tag::String("héllo".into())),
            ("empty", Tag::List(0, vec![])),
            ("list", Tag::List(3, vec![Tag::Int(1), Tag::Int(2)])),
            ("nested", compound([("x", Tag::Int(5))])),
            ("ia", Tag::IntArray(vec![1, -2])),
            ("la", Tag::LongArray(vec![i64::MIN, 0])),
        ])
    }

    #[test]
    fn round_trips_both_endians() {
        for endian in [Endian::Little, Endian::Big] {
            let mut buf = Vec::new();
            write(&mut buf, "root", &sample(), endian);
            let (name, tag, used) = read(&buf, endian).unwrap();
            assert_eq!(name, "root");
            assert_eq!(tag, sample());
            assert_eq!(used, buf.len());
        }
    }

    #[test]
    fn reads_concatenated_roots() {
        let mut buf = Vec::new();
        write(&mut buf, "", &compound([("a", Tag::Int(1))]), Endian::Little);
        write(&mut buf, "", &compound([("a", Tag::Int(2))]), Endian::Little);
        let all = read_all(&buf, Endian::Little).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].get("a"), Some(&Tag::Int(2)));
    }

    #[test]
    fn truncated_input_errors() {
        let mut buf = Vec::new();
        write(&mut buf, "root", &sample(), Endian::Big);
        assert!(read(&buf[..buf.len() - 1], Endian::Big).is_err());
    }
}
