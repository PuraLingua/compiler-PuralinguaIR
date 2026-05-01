use std::{fmt::Display, marker::ConstParamTy};

use compiler_base::{pura_lingua::global::StrEnum, utility::uni_lexer::TokenInfo};
use konst::const_panic::PanicFmt;
use proc_macro::DeriveEnumAlias;

use crate::{TokenKind, special_char_impl_token_info_alias};

#[allow(unused)]
#[derive(Debug, Copy, Clone, ConstParamTy, StrEnum, PanicFmt, Hash, DeriveEnumAlias)]
#[derive_const(PartialEq, Eq)]
// cSpell:disable-next-line
#[pfmt(crate = konst::const_panic)]
#[global_crate = "compiler_base::pura_lingua::global"]
#[alias_of(TokenInfo<TokenKind>)]
#[alias_suffix = "Char"]
#[alias_derive(Copy, Hash)]
#[alias_const_derive(Clone, PartialEq, Eq)]
#[implement(special_char_impl_token_info_alias)]
pub enum SpecialChar {
    #[str_val("(")]
    /// (
    ParenthesisOpen,
    #[str_val(")")]
    /// )
    ParenthesisClose,
    #[str_val("[")]
    /// [
    BracketOpen,
    #[str_val("]")]
    /// ]
    BracketClose,
    #[str_val("{")]
    /// {
    BraceOpen,
    #[str_val("}")]
    /// }
    BraceClose,
    #[str_val("<")]
    /// <
    LessThanSign,
    #[str_val(">")]
    /// >
    GreaterThanSign,

    #[str_val(":")]
    /// :
    Colon,
    #[str_val(";")]
    /// ;
    Semicolon,
    #[str_val("=")]
    /// =
    EqualsSign,

    #[str_val("_")]
    /// _
    Underscore,

    #[str_val(",")]
    /// ,
    Comma,
    #[str_val(".")]
    /// .
    Period,

    #[str_val("!")]
    /// !
    ExclamationMark,
    #[str_val("@")]
    /// @
    At,
    #[str_val("?")]
    /// ?
    QuestionMark,
    #[str_val("&")]
    /// &
    Ampersand,

    #[str_val("+")]
    /// `+`
    Plus,
    #[str_val("-")]
    /// `-`
    Hyphen,
    #[str_val("~")]
    /// ~
    TildeSymbol,
    #[str_val("^")]
    /// ^
    Circumflex,
    #[str_val("*")]
    /// `*`
    Asterisk,
    #[str_val("/")]
    #[doc(alias = "Slash")]
    /// /
    Solidus,
    #[str_val("%")]
    /// %
    PercentSign,
    #[str_val("|")]
    #[doc(alias = "Pipe")]
    /// |
    VerticalBar,
    #[str_val("#")]
    /// \#
    NumberSign,

    // combined
    #[str_val("=>")]
    /// =>
    Arrow,
    #[str_val("->")]
    /// ->
    ThinArrow,

    #[str_val("::")]
    /// ::
    DoubledColon,
    #[str_val("++")]
    /// ++
    DoubledPlus,
    #[str_val("--")]
    /// --
    DoubledHyphen,
    #[str_val("..")]
    /// ..
    DoubledDot,
    #[str_val("==")]
    /// ==
    DoubledEqualsSign,
    #[str_val("&&")]
    /// &&
    DoubledAmpersand,
    #[str_val("||")]
    /// ||
    DoubledVerticalBar,
}

impl SpecialChar {
    pub fn maybe_parsable(c: char) -> bool {
        Self::VARIANTS.iter().any(|x| x.as_str().starts_with(c))
    }

    pub fn parse_prefix(s: &str) -> Option<SpecialChar> {
        Self::VARIANTS_ORDERED_REV
            .iter()
            .find(|x| s.starts_with(x.as_str()))
            .copied()
    }

    pub fn get_back_pair(&self) -> Option<Self> {
        match self {
            SpecialChar::BraceOpen => Some(SpecialChar::BraceClose),
            SpecialChar::BracketOpen => Some(SpecialChar::BracketClose),
            SpecialChar::ParenthesisOpen => Some(SpecialChar::ParenthesisClose),

            _ => None,
        }
    }

    pub fn get_front_pair(&self) -> Option<Self> {
        match self {
            SpecialChar::BraceClose => Some(SpecialChar::BraceOpen),
            SpecialChar::BracketClose => Some(SpecialChar::BracketOpen),
            SpecialChar::ParenthesisClose => Some(SpecialChar::ParenthesisOpen),

            _ => None,
        }
    }
}

impl Display for SpecialChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl const PartialEq<str> for SpecialChar {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}
