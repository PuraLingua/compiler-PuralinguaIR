use compiler_base::{
    abstract_info::type_reference::TypeReference,
    utility::uni_parser::{IParse, ParseError, ParseStream},
};
use token::{SpecialChar, TokenKind};

use crate::identifier::Identifier;

#[derive(Clone, Debug)]
pub enum GenericBound {
    Type(TypeReference),
}

pub struct ParseGenericResult {
    pub generics: Vec<Identifier>,
    pub is_generic_infinite: bool,
}

impl IParse for ParseGenericResult {
    type TTokenKind = TokenKind;

    fn parse<'a>(
        stream: &mut ParseStream<'a, Self::TTokenKind>,
    ) -> Result<Self, ParseError<Self::TTokenKind>> {
        let mut generics = Vec::new();
        let mut is_generic_infinite = false;

        if stream.expect(SpecialChar::BracketOpen).is_err() {
            return Ok(ParseGenericResult {
                generics,
                is_generic_infinite,
            });
        }

        while let Ok(g) = stream.parse::<Identifier>() {
            generics.push(g);
        }

        if stream.expect(SpecialChar::Plus).is_ok() {
            is_generic_infinite = true;
        }

        stream.expect(SpecialChar::BracketClose)?;

        Ok(Self {
            generics,
            is_generic_infinite,
        })
    }
}
