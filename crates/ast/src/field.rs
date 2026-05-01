use compiler_base::{
    abstract_info::{self, IToAbstract, type_reference::TypeReference},
    global::attrs::{FieldImplementationFlags, Visibility},
    pura_lingua::global::attrs::FieldAttr,
    utility::uni_parser::{ParseError, ParseStream, proc_macros::Parse},
};
use enumflags2::BitFlags;
use token::{IntegerToken, Keyword, SpecialChar, TokenKind};

use crate::identifier::Identifier;

#[derive(Clone, Debug)]
pub struct Field {
    pub attr: FieldAttr,
    pub index: u32,
    pub name: Identifier,
    pub ty: TypeReference,
}

impl IToAbstract for Field {
    type Abstract = abstract_info::type_information::field::Field;

    type Err = !;

    fn to_abstract(&self) -> Result<Self::Abstract, Self::Err> {
        Ok(Self::Abstract {
            name: self.name.var.clone(),
            id: self.index,
            attr: self.attr,
            ty: self.ty.clone(),
        })
    }
}

impl Field {
    pub fn parse<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
        ty_generics: &[Identifier],
    ) -> Result<Self, ParseError<TokenKind>> {
        debug_assert!(stream.consume().is_some_and(|x| x.kind == Keyword::Field));

        stream.expect(SpecialChar::BracketOpen)?;
        let (attr, index) = stream.call(attr)?;
        stream.expect(SpecialChar::BracketClose)?;

        let name = stream.parse::<Identifier>()?;

        stream.expect(SpecialChar::Colon)?;

        let ty = stream.call(|stream| crate::ty::type_reference(stream, ty_generics, &[]))?;

        stream.expect(SpecialChar::Semicolon)?;

        Ok(Self {
            attr,
            index,
            name,
            ty,
        })
    }
}

#[derive(Parse, Clone, Debug)]
#[t_token_kind(TokenKind)]
pub enum FieldReference {
    ByIndex(
        #[custom({
            let _0 = stream.parse::<IntegerToken>()?.get_value(stream.source()).unwrap().try_into().unwrap();
        })]
        u32,
    ),
    ByName(Identifier),
}

fn attr<'a>(
    stream: &mut compiler_base::utility::uni_parser::ParseStream<'a, TokenKind>,
) -> Result<(FieldAttr, u32), compiler_base::utility::uni_parser::ParseError<TokenKind>> {
    let mut result = FieldAttr::new(Visibility::Private, BitFlags::empty());
    let mut index = None;

    while let Ok(token) = stream.require_f(|x| {
        matches!(x.kind, TokenKind::Keyword(kw) if kw.is_field_modifier()) || x.kind.is_int()
    }) {
        if token.kind.is_int() {
            index = Some(IntegerToken(token));
            continue;
        }
        let TokenKind::Keyword(tk_kw) = token.kind else {
            unreachable!();
        };
        if !tk_kw.is_field_modifier() {
            unreachable!();
        }
        match tk_kw {
            token::Keyword::Public => {
                result.set_vis(Visibility::Public);
            }
            token::Keyword::Private => {
                result.set_vis(Visibility::Private);
            }
            token::Keyword::Internal => {
                result.set_vis(Visibility::AssemblyOnly);
            }
            token::Keyword::Static => {
                result
                    .impl_flags_mut()
                    .insert(FieldImplementationFlags::Static);
            }

            _ => unreachable!(),
        }
    }

    let index = index.and_then(|x| {
        x.get_value(stream.source())
            .map(u32::try_from)
            .map(Result::ok)
            .flatten()
    });

    Ok((
        result,
        index.ok_or(ParseError::ExpectLiteral(token::LiteralKindType::Int))?,
    ))
}
