use std::fmt::Display;

use compiler_base::utility::uni_parser::{IParse, ParseError, ParseStream};
use token::TokenKind;

#[derive(Clone, Debug)]
pub struct Identifier {
    pub var: String,
}

impl Identifier {
    pub const fn new<T: [const] Into<String>>(data: T) -> Self {
        Self { var: data.into() }
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        &self.var
    }
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.var == other.var
    }
}

impl PartialEq<String> for Identifier {
    fn eq(&self, other: &String) -> bool {
        self.var.eq(other)
    }
}

impl PartialEq<str> for Identifier {
    fn eq(&self, other: &str) -> bool {
        self.var.eq(other)
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <_ as Display>::fmt(&self.var, f)
    }
}

impl IParse for Identifier {
    type TTokenKind = TokenKind;

    fn parse<'a>(
        stream: &mut ParseStream<'a, Self::TTokenKind>,
    ) -> Result<Self, ParseError<Self::TTokenKind>> {
        let current = *stream.current().ok_or(ParseError::UnexpectedEOF)?;
        match current.kind {
            token::TokenKind::Literal { kind, suffix_start } => match kind {
                token::LiteralKind::Str { terminated: true } => {
                    let s =
                        &stream.token_stream().src[current.span][1..(suffix_start as usize - 1)];
                    stream.advance();
                    Ok(Identifier::new(
                        compiler_base::descape::UnescapeExt::to_unescaped(s)?.into_owned(),
                    ))
                }
                token::LiteralKind::RawStr { n_hashes } => {
                    let prefix_offset = 2 + n_hashes.unwrap_or(0) as usize;
                    let until = suffix_start as usize - 1 - n_hashes.unwrap_or(0) as usize;
                    stream.advance();
                    Ok(Identifier::new(
                        stream.token_stream().src[current.span][prefix_offset..until].to_owned(),
                    ))
                }
                _ => Err(ParseError::Unexpect(current)),
            },
            token::TokenKind::Identifier => {
                stream.advance();
                Ok(Identifier::new(
                    stream.token_stream().src[current.span].to_owned(),
                ))
            }
            token::TokenKind::RawIdentifier => {
                stream.advance();
                Ok(Identifier::new(
                    stream.token_stream().src[current.span][2..].to_owned(),
                ))
            }
            _ => Err(ParseError::Unexpect(current)),
        }
    }
}
