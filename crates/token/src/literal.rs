use std::{fmt::Display, marker::ConstParamTy};

use compiler_base::pura_lingua::global::WithType;
use konst::const_panic::PanicFmt;

#[allow(unused)]
#[derive(Debug, Copy, PanicFmt, WithType, Hash)]
#[derive_const(Clone, PartialEq, Eq)]
#[with_type(derive = (Eq, Copy, Debug, derive_more::Display))]
#[with_type(derive_const = (Clone, PartialEq))]
// cSpell:disable-next-line
#[pfmt(crate = konst::const_panic)]
pub enum LiteralKind {
    /// `12_u8`, `0o100`, `0b120i99`, `1f32`.
    Int { base: Base, empty_int: bool },
    /// `12.34f32`, `1e3`, but not `1f32`.
    Float { base: Base, empty_exponent: bool },
    /// `'a'`, `'\\'`, `'''`, `';`
    Char { terminated: bool },
    /// `b'a'`, `b'\\'`, `b'''`, `b';`
    Byte { terminated: bool },
    /// `"abc"`, `"abc`
    Str { terminated: bool },
    /// `b"abc"`, `b"abc`
    ByteStr { terminated: bool },
    /// `r"abc"`, `r#"abc"#`, `r####"ab"###"c"####`, `r#"a`. `None` indicates
    /// an invalid literal.
    RawStr { n_hashes: Option<u8> },
    /// `br"abc"`, `br#"abc"#`, `br####"ab"###"c"####`, `br#"a`. `None`
    /// indicates an invalid literal.
    RawByteStr { n_hashes: Option<u8> },
}

impl Display for LiteralKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralKind::Int { base, empty_int } => write!(f, "Int({base}, is_empty:{empty_int})"),
            LiteralKind::Float {
                base,
                empty_exponent,
            } => write!(f, "Float({base}, is_empty_exponent:{empty_exponent})"),
            LiteralKind::Char { terminated } => write!(f, "Char({terminated})"),
            LiteralKind::Byte { terminated } => write!(f, "Byte({terminated})"),
            LiteralKind::Str { terminated } => write!(f, "Str({terminated})"),
            LiteralKind::ByteStr { terminated } => write!(f, "ByteStr({terminated})"),
            LiteralKind::RawStr { n_hashes } => write!(
                f,
                "RawStr({})",
                if let Some(n_hashes) = n_hashes {
                    n_hashes.to_string()
                } else {
                    "".to_owned()
                }
            ),
            LiteralKind::RawByteStr { n_hashes } => write!(
                f,
                "RawByteStr({})",
                if let Some(n_hashes) = n_hashes {
                    n_hashes.to_string()
                } else {
                    "".to_owned()
                }
            ),
        }
    }
}

/// Base of numeric literal encoding according to its prefix.
#[allow(unused)]
#[derive(Debug, Copy, ConstParamTy, PanicFmt, Hash)]
#[derive_const(Clone, PartialEq, Eq)]
#[pfmt(crate = konst::const_panic)]
pub enum Base {
    /// Literal starts with "0b".
    Binary = 2,
    /// Literal starts with "0o".
    Octal = 8,
    /// Literal doesn't contain a prefix.
    Decimal = 10,
    /// Literal starts with "0x".
    Hexadecimal = 16,
}

impl Display for Base {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Base::Binary => write!(f, "Binary"),
            Base::Octal => write!(f, "Octal"),
            Base::Decimal => write!(f, "Decimal"),
            Base::Hexadecimal => write!(f, "Hexadecimal"),
        }
    }
}

/// `#"abc"#`, `##"a"` (fewer closing), or even `#"a` (unterminated).
///
/// Can capture fewer closing hashes than starting hashes,
/// for more efficient lexing and better backwards diagnostics.
#[derive(Copy, Debug)]
#[derive_const(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GuardedStr {
    pub n_hashes: u32,
    pub terminated: bool,
    pub token_len: u32,
}
