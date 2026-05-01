use std::borrow::Cow;

use compiler_base::utility::{
    uni_lexer::TokenInfo,
    uni_parser::{IParse, ParseError, ParseStream},
};

use crate::{LiteralKind, LiteralKindType, TokenKind};

macro token_info_aliases($($(#[$meta:meta])* $i:ident)*) {$(
	#[repr(transparent)]
    #[derive(Copy, Hash)]
	#[derive_const(Clone, PartialEq, Eq)]
	$(#[$meta])*
	pub struct $i(pub ::compiler_base::utility::uni_lexer::TokenInfo<$crate::TokenKind>);

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
			fn kind(&self) -> &Self::Kind {
				&self.0.kind
			}
			fn span(&self) -> &compiler_base::span::Span {
				&self.0.span
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

		impl std::ops::DerefMut for $i {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.0
			}
		}

		impl std::ops::Deref for $i {
			type Target = TokenInfo<TokenKind>;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}
	};
)*}

token_info_aliases! {
    IdentifierToken
    CharToken
    /// String or RawString
    StringToken
    ByteToken
    /// ByteString or RawByteString
    ByteStringToken
    IntegerToken

    ClassModifierToken
    StructModifierToken
    MethodModifierToken
    FieldModifierToken
}

impl IParse for IdentifierToken {
    type TTokenKind = TokenKind;

    fn parse<'a>(
        stream: &mut ParseStream<'a, Self::TTokenKind>,
    ) -> Result<Self, ParseError<Self::TTokenKind>> {
        stream.require_f(|x| x.kind.is_identifier()).map(Self)
    }
}

macro impl_parse4modifier($($Ty:ident if $checker:ident),* $(,)?) {$(
	impl IParse for $Ty {
		type TTokenKind = TokenKind;

		fn parse<'a>(
			stream: &mut ParseStream<'a, Self::TTokenKind>,
		) -> Result<Self, ParseError<Self::TTokenKind>> {
			stream
				.require_f(|x| matches!(x.kind, TokenKind::Keyword(kw) if kw.$checker()))
				.map(Self)
		}
	}
)*}

impl_parse4modifier! {
    ClassModifierToken if is_class_modifier,
    StructModifierToken if is_struct_modifier,
    MethodModifierToken if is_method_modifier,
    FieldModifierToken if is_field_modifier,
}

macro impl_parse4lit($($Ty:ident in [$($kind:expr),* $(,)?]),* $(,)?) {$(
	impl IParse for $Ty {
		type TTokenKind = TokenKind;

		fn parse<'a>(
			stream: &mut ParseStream<'a, Self::TTokenKind>,
		) -> Result<Self, ParseError<Self::TTokenKind>> {
			stream
				.require_f(|x| false $(|| x.kind == $kind)*)
				.map(Self)
		}
	}
)*}

impl_parse4lit! {
    CharToken in [LiteralKindType::Char],
    StringToken in [LiteralKindType::Str, LiteralKindType::RawStr],
    ByteStringToken in [LiteralKindType::ByteStr, LiteralKindType::RawByteStr],
    ByteToken in [LiteralKindType::Byte],
    IntegerToken in [LiteralKindType::Int],
}

impl CharToken {
    pub fn get_value(&self, src: &str) -> Option<char> {
        match self.kind {
            TokenKind::Literal {
                kind: LiteralKind::Char { terminated },
                suffix_start,
            } => {
                assert!(terminated);
                let s = &src[self.span][1..(suffix_start as usize - 1)];
                compiler_base::descape::UnescapeExt::to_unescaped(s)
                    .ok()
                    .and_then(|x| x.chars().next())
            }
            _ => None,
        }
    }
}

impl StringToken {
    pub fn is_raw(&self) -> bool {
        matches!(
            &**self,
            TokenInfo {
                kind: TokenKind::Literal {
                    kind: LiteralKind::RawStr { .. },
                    ..
                },
                ..
            }
        )
    }
    pub fn get_value<'a>(&self, src: &'a str) -> Option<Cow<'a, str>> {
        match self.kind {
            TokenKind::Literal { kind, suffix_start } => match kind {
                LiteralKind::Str { terminated } => {
                    assert!(terminated);
                    let s = &src[self.span][1..(suffix_start as usize - 1)];
                    compiler_base::descape::UnescapeExt::to_unescaped(s).ok()
                }
                LiteralKind::RawStr { n_hashes } => {
                    let prefix_offset = 2 + n_hashes.unwrap_or(0) as usize;
                    let until = suffix_start as usize - 1 - n_hashes.unwrap_or(0) as usize;
                    Some(Cow::Borrowed(&src[self.span][prefix_offset..until]))
                }
                _ => None,
            },
            _ => None,
        }
    }
}

impl ByteToken {
    pub fn get_value(&self, src: &str) -> Option<u8> {
        match self.kind {
            TokenKind::Literal {
                kind: LiteralKind::Byte { terminated },
                suffix_start,
            } => {
                assert!(terminated);
                let s = &src[self.span][1..(suffix_start as usize - 1)];
                compiler_base::descape::UnescapeExt::to_unescaped(s)
                    .ok()
                    .and_then(|x| x.as_bytes().first().copied())
            }
            _ => None,
        }
    }
}

impl ByteStringToken {
    pub fn is_raw(&self) -> bool {
        matches!(
            &**self,
            TokenInfo {
                kind: TokenKind::Literal {
                    kind: LiteralKind::RawByteStr { .. },
                    ..
                },
                ..
            }
        )
    }
    pub fn get_value<'a>(&self, src: &'a str) -> Option<Cow<'a, [u8]>> {
        match self.kind {
            TokenKind::Literal { kind, suffix_start } => match kind {
                LiteralKind::ByteStr { terminated } => {
                    assert!(terminated);
                    let s = &src[self.span][1..(suffix_start as usize - 1)];
                    compiler_base::descape::UnescapeExt::to_unescaped(s)
                        .ok()
                        .map(|x| match x {
                            Cow::Borrowed(x) => Cow::Borrowed(x.as_bytes()),
                            Cow::Owned(x) => Cow::Owned(x.into_bytes()),
                        })
                }
                LiteralKind::RawByteStr { n_hashes } => {
                    let prefix_offset = 2 + n_hashes.unwrap_or(0) as usize;
                    let until = suffix_start as usize - 1 - n_hashes.unwrap_or(0) as usize;
                    Some(Cow::Borrowed(
                        &src[self.span].as_bytes()[prefix_offset..until],
                    ))
                }
                _ => None,
            },
            _ => None,
        }
    }
}

impl IntegerToken {
    pub fn get_value(&self, src: &str) -> Option<u64> {
        let TokenInfo {
            kind:
                TokenKind::Literal {
                    kind:
                        LiteralKind::Int {
                            base,
                            empty_int: false,
                        },
                    suffix_start,
                },
            span,
        } = &**self
        else {
            return None;
        };
        let value_s = &src[*span][..(*suffix_start as usize)];
        match base {
            crate::Base::Binary => u64::from_str_radix(&value_s[2..], 2).ok(),
            crate::Base::Octal => u64::from_str_radix(&value_s[2..], 8).ok(),
            crate::Base::Decimal => value_s.parse::<u64>().ok(),
            crate::Base::Hexadecimal => u64::from_str_radix(&value_s[2..], 16).ok(),
        }
    }
}
