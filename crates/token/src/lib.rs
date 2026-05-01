#![feature(decl_macro)]
#![feature(derive_const)]
#![feature(const_clone)]
#![feature(const_cmp)]
#![feature(const_trait_impl)]
#![feature(const_convert)]
#![feature(min_adt_const_params)]

use std::fmt::Display;

use compiler_base::{
    global::UnwrapEnum,
    utility::uni_lexer::{Cursor, ITokenKind, TokenInfo},
};
use konst::const_panic::PanicFmt;

mod comment;
mod keyword;
mod literal;
mod parse;
mod special_char;

pub use comment::*;
pub use keyword::*;
pub use literal::*;
pub use special_char::*;

#[allow(unused)]
#[derive(Debug, Copy, PanicFmt, Hash, UnwrapEnum)]
#[derive_const(Clone, PartialEq, Eq)]
// cSpell:disable-next-line
#[pfmt(crate = konst::const_panic)]
#[unwrap_enum(ref, ref_mut, owned)]
pub enum TokenKind {
    Keyword(Keyword),
    StatementKeyword(StatementKeyword),
    SpecialChar(SpecialChar),
    Comment(Comment),
    Literal {
        kind: LiteralKind,
        suffix_start: u32,
    },
    GuardedStrPrefix,

    /// Maybe including keywords
    Identifier,
    InvalidIdentifier,
    RawIdentifier,
    UnknownPrefix,

    WhiteSpace,
    Unknown,
    Eof,
}

impl TokenKind {
    pub fn is_identifier(&self) -> bool {
        [Self::Identifier, Self::RawIdentifier].contains(self)
    }
    pub fn is_int(&self) -> bool {
        matches!(
            self,
            TokenKind::Literal {
                kind: LiteralKind::Int {
                    base: _,
                    empty_int: false,
                },
                suffix_start: _,
            }
        )
    }
}

impl const PartialEq<LiteralKindType> for TokenKind {
    fn eq(&self, other: &LiteralKindType) -> bool {
        matches!(
            self,
            Self::Literal { kind: got, .. } if got.to_type().eq(other)
        )
    }
}

impl const PartialEq<SpecialChar> for TokenKind {
    fn eq(&self, other: &SpecialChar) -> bool {
        Self::SpecialChar(*other).eq(self)
    }
}

impl const PartialEq<Keyword> for TokenKind {
    fn eq(&self, other: &Keyword) -> bool {
        Self::Keyword(*other).eq(self)
    }
}

impl ITokenKind for TokenKind {
    type LiteralKindType = LiteralKindType;

    const EOF: Self = TokenKind::Eof;
    const UNKNOWN: Self = TokenKind::Unknown;

    fn is_whitespace(self) -> bool {
        self == Self::WhiteSpace
    }
    const WS_TAB: Self = Self::WhiteSpace;
    const WS_NEW_LINE: Self = Self::WhiteSpace;
    const WS_VERTICAL_TAB: Self = Self::WhiteSpace;
    const WS_FORM_FEED: Self = Self::WhiteSpace;
    const WS_CARRIAGE_RETURN: Self = Self::WhiteSpace;
    const WS_SPACE: Self = Self::WhiteSpace;

    const WS_LATIN1_NEXT_LINE: Self = Self::WhiteSpace;

    const WS_BIDI_LEFT_TO_RIGHT_MARK: Self = Self::WhiteSpace;
    const WS_BIDI_RIGHT_TO_LEFT_MARK: Self = Self::WhiteSpace;

    const WS_LINE_SEPARATOR: Self = Self::WhiteSpace;
    const WS_PARAGRAPH_SEPARATOR: Self = Self::WhiteSpace;

    fn parse(cursor: &mut Cursor<'_, Self>, prev_pos: usize, first_char: char) -> Option<Self> {
        parse::parse(cursor, prev_pos, first_char)
    }

    fn is_invalid(self) -> bool {
        match self {
            TokenKind::Literal {
                kind,
                suffix_start: _,
            } => match kind {
                LiteralKind::Char { terminated }
                | LiteralKind::Byte { terminated }
                | LiteralKind::Str { terminated }
                | LiteralKind::ByteStr { terminated }
                    if !terminated =>
                {
                    true
                }
                _ => false,
            },
            TokenKind::InvalidIdentifier | TokenKind::Unknown => true,
            _ => false,
        }
    }

    fn edit(cursor: &Cursor<'_, Self>, tk: &mut TokenInfo<Self>) -> bool {
        if tk.kind == TokenKind::Identifier {
            if let Ok(kw) = Keyword::try_from(&cursor.original_source()[tk.span]) {
                tk.kind = TokenKind::Keyword(kw);
            } else if let Ok(kw) = StatementKeyword::try_from(&cursor.original_source()[tk.span]) {
                tk.kind = TokenKind::StatementKeyword(kw);
            }
        }

        !(tk.kind.is_whitespace() || matches!(tk.kind, TokenKind::Comment(_)))
    }
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Keyword(keyword) => Display::fmt(keyword, f),
            TokenKind::StatementKeyword(keyword) => Display::fmt(keyword, f),
            TokenKind::SpecialChar(special_char) => Display::fmt(special_char, f),
            TokenKind::Comment(comment) => Display::fmt(comment, f),

            TokenKind::Literal { kind, suffix_start } => {
                write!(f, "Literal({}, {})", kind, suffix_start)
            }
            TokenKind::GuardedStrPrefix => write!(f, "GuardedStrPrefix"),

            TokenKind::Identifier => write!(f, "Identifier"),
            TokenKind::InvalidIdentifier => write!(f, "InvalidIdentifier"),
            TokenKind::RawIdentifier => write!(f, "RawIdentifier"),

            TokenKind::UnknownPrefix => write!(f, "UnknownPrefix"),
            TokenKind::WhiteSpace => write!(f, "WhiteSpace"),
            TokenKind::Unknown => write!(f, "Unknown"),
            TokenKind::Eof => write!(f, "Eof"),
        }
    }
}

trait TokenSealed {}

#[expect(private_bounds)]
pub trait TokenName: TokenSealed {
    const TOKEN_NAME: &str;
}

macro common_impl_token_info_alias($i:ident) {
    const _: () = {
        type __TokenInfo = ::compiler_base::utility::uni_lexer::TokenInfo<$crate::TokenKind>;
        impl const ::core::convert::From<__TokenInfo> for $i {
            fn from(v: __TokenInfo) -> Self {
                $i(v)
            }
        }

        impl ::std::fmt::Debug for $i {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                <__TokenInfo as ::std::fmt::Debug>::fmt(&self.0, f)
            }
        }

        impl ::std::fmt::Display for $i {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                <__TokenInfo as ::std::fmt::Display>::fmt(&self.0, f)
            }
        }

        impl const ::compiler_base::utility::uni_lexer::IToken for $i {
            type Kind = $crate::TokenKind;
            fn kind(&self) -> &$crate::TokenKind {
                &self.0.kind()
            }
            fn span(&self) -> &compiler_base::span::Span {
                &self.0.span()
            }
        }

        impl fmt_with_src::DebugWithSrc for $i {
            type Debug<'a> = ::compiler_base::utility::uni_lexer::TokenDebug<'a, $i>;
            fn debug<'a>(&'a self, src: &'a str) -> Self::Debug<'a> {
                ::compiler_base::utility::uni_lexer::TokenDebug(self, src)
            }
        }

        impl fmt_with_src::DisplayWithSrc for $i {
            type Display<'a> = ::compiler_base::utility::uni_lexer::TokenDisplay<'a, $i>;
            fn display<'a>(&'a self, src: &'a str) -> Self::Display<'a> {
                ::compiler_base::utility::uni_lexer::TokenDisplay(self, src)
            }
        }

        impl $crate::TokenSealed for $i {}

        impl $crate::TokenName for $i {
            const TOKEN_NAME: &str = stringify!($i);
        }
    };
}

macro special_char_impl_token_info_alias($i:ident, $name:ident) {
    $crate::common_impl_token_info_alias!($i);
    const _: () = {
        type __TokenInfo = ::compiler_base::utility::uni_lexer::TokenInfo<$crate::TokenKind>;

        impl ::compiler_base::utility::uni_parser::IParse for $i {
            type TTokenKind = $crate::TokenKind;

            fn parse<'a>(
                stream: &mut ::compiler_base::utility::uni_parser::ParseStream<
                    'a,
                    Self::TTokenKind,
                >,
            ) -> Result<Self, ::compiler_base::utility::uni_parser::ParseError<Self::TTokenKind>>
            {
                stream
                    .require($crate::TokenKind::SpecialChar($crate::SpecialChar::$name))
                    .map($i)
            }
        }
    };
}

mod token_aliases;
pub use token_aliases::*;
