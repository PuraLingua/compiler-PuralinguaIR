use compiler_base::{
    abstract_info::{
        IToAbstract, type_information::TypeInformation, type_reference::TypeReference,
    },
    global::attrs::{
        ClassImplementationFlags, StructImplementationFlags, TypeAttr, TypeSpecificAttr,
        TypeSpecificAttrType, Visibility,
    },
    utility::uni_parser::{self, IParse, ParseError, ParseStream},
};
use enumflags2::BitFlags;
use token::{IntegerToken, Keyword, LiteralKindType, SpecialChar, TokenKind};

use crate::{
    field::Field,
    identifier::Identifier,
    method::Method,
    ty::{class::ClassDef, interface::InterfaceDef, r#struct::StructDef},
};

pub mod class;
pub mod interface;
pub mod r#struct;

#[derive(Clone, Debug)]
pub enum TypeDef {
    Class(ClassDef),
    Struct(StructDef),
    Interface(InterfaceDef),
}

impl IToAbstract for TypeDef {
    type Abstract = TypeInformation;

    type Err = !;

    fn to_abstract(&self) -> Result<Self::Abstract, Self::Err> {
        match self {
            TypeDef::Class(class_def) => class_def.to_abstract(),
            TypeDef::Struct(struct_def) => struct_def.to_abstract(),
            TypeDef::Interface(interface_def) => interface_def.to_abstract(),
        }
    }
}

impl TypeDef {
    pub fn name(&self) -> &str {
        match self {
            TypeDef::Class(class_def) => class_def.name.var.as_ref(),
            TypeDef::Struct(struct_def) => struct_def.name.var.as_ref(),
            TypeDef::Interface(interface_def) => interface_def.name.var.as_ref(),
        }
    }
    pub fn parent(&self) -> Option<&TypeReference> {
        match self {
            TypeDef::Class(class_def) => class_def.parent.as_ref(),
            TypeDef::Struct(_) => None,
            TypeDef::Interface(_) => None,
        }
    }
    pub fn set_index(&mut self, index: u32) {
        match self {
            TypeDef::Class(class_def) => class_def.index = index,
            TypeDef::Struct(struct_def) => struct_def.index = index,
            TypeDef::Interface(interface_def) => interface_def.index = index,
        }
    }
    pub fn index(&self) -> u32 {
        match self {
            TypeDef::Class(class_def) => class_def.index,
            TypeDef::Struct(struct_def) => struct_def.index,
            TypeDef::Interface(interface_def) => interface_def.index,
        }
    }
    pub fn sort_items(&mut self) {
        match self {
            TypeDef::Class(class_def) => class_def.sort_items(),
            TypeDef::Struct(struct_def) => struct_def.sort_items(),
            TypeDef::Interface(interface_def) => interface_def.sort_items(),
        }
    }
    pub fn methods(&self) -> &[Method] {
        match self {
            TypeDef::Class(class_def) => &class_def.methods,
            TypeDef::Struct(struct_def) => &struct_def.methods,
            TypeDef::Interface(interface_def) => &interface_def.methods,
        }
    }
    pub fn fields(&self) -> &[Field] {
        match self {
            TypeDef::Class(class_def) => &class_def.fields,
            TypeDef::Struct(struct_def) => &struct_def.fields,
            TypeDef::Interface(_) => &[],
        }
    }
}

impl IParse for TypeDef {
    type TTokenKind = TokenKind;

    fn parse<'a>(
        stream: &mut ParseStream<'a, Self::TTokenKind>,
    ) -> Result<Self, ParseError<Self::TTokenKind>> {
        let tok = stream.current().ok_or(ParseError::UnexpectedEOF)?;
        match tok.kind {
            TokenKind::Keyword(Keyword::Class) => stream.parse().map(TypeDef::Class),
            TokenKind::Keyword(Keyword::Struct) => stream.parse().map(TypeDef::Struct),
            TokenKind::Keyword(Keyword::Interface) => stream.parse().map(TypeDef::Interface),

            _ => Err(ParseError::Unexpect(*tok).into()),
        }
    }
}

pub fn type_reference<'a>(
    stream: &mut ParseStream<'a, TokenKind>,
    ty_generics: &[Identifier],
    method_generics: &[Identifier],
) -> Result<TypeReference, ParseError<TokenKind>> {
    if stream.require(SpecialChar::BracketOpen).is_ok() {
        let assembly_name = stream
            .require(SpecialChar::ExclamationMark)
            .map(|x| stream.token_stream().src[x.span].to_owned())
            .or_else(|_| stream.parse::<Identifier>().map(|x| x.var))?;
        stream.require(SpecialChar::BracketClose)?;
        let type_name = stream.parse::<Identifier>()?;
        if stream.require(SpecialChar::LessThanSign).is_ok() {
            let generics = uni_parser::punctuated::punctuated::<TokenKind, TypeReference, _, _, _>(
                stream,
                false,
                |stream| type_reference(stream, ty_generics, method_generics),
                |stream| stream.require(SpecialChar::Comma).map_err(From::from),
            )?;
            stream.require(SpecialChar::GreaterThanSign)?;
            Ok(TypeReference::Common {
                assembly_name,
                name: type_name.var,
                index: None,
                generics: generics.into_iter().collect(),
            })
        } else {
            Ok(TypeReference::Common {
                assembly_name,
                name: type_name.var,
                index: None,
                generics: Vec::new(),
            })
        }
    } else {
        let ident = stream.parse::<Identifier>()?;
        if let Some(pos) = ty_generics.iter().position(|x| x.var == ident.var) {
            return Ok(TypeReference::TypeGeneric(Some(ident.var), pos as u32));
        }

        if let Some(pos) = method_generics.iter().position(|x| x.var == ident.var) {
            return Ok(TypeReference::MethodGeneric(Some(ident.var), pos as u32));
        } else {
            return Err(ParseError::Unexpect(
                stream.token_stream().tokens[stream.get_index() - 1],
            ));
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct TypeAttrAndIndex<const TYPE: TypeSpecificAttrType> {
    pub attr: TypeAttr,
    pub index: IntegerToken,
}

impl<const TYPE: TypeSpecificAttrType> IParse for TypeAttrAndIndex<TYPE> {
    type TTokenKind = TokenKind;

    fn parse<'a>(
        stream: &mut ParseStream<'a, Self::TTokenKind>,
    ) -> Result<Self, ParseError<Self::TTokenKind>> {
        let mut result = TypeAttr::new(
            Visibility::Private,
            match TYPE {
                TypeSpecificAttrType::Class => TypeSpecificAttr::Class(BitFlags::empty()),
                TypeSpecificAttrType::Struct => TypeSpecificAttr::Struct(BitFlags::empty()),
                TypeSpecificAttrType::Interface => TypeSpecificAttr::Interface(BitFlags::empty()),
            },
        );
        let mut index = None;

        while let Ok(token) = stream.require_f(|x| match TYPE {
            TypeSpecificAttrType::Class => matches!(x.kind, TokenKind::Keyword(kw) if kw.is_class_modifier()),
            TypeSpecificAttrType::Struct => matches!(x.kind, TokenKind::Keyword(kw) if kw.is_struct_modifier()),
            TypeSpecificAttrType::Interface => matches!(x.kind, TokenKind::Keyword(kw) if kw.is_interface_modifier()),
        } || x.kind.is_int()) {
            if token.kind.is_int() {
                index = Some(IntegerToken(token));
                continue;
            }

            let TokenKind::Keyword(kw) = token.kind else { unreachable!() };

            match TYPE {
                TypeSpecificAttrType::Class => match kw {
                    Keyword::Public => {
                        result.set_vis(Visibility::Public);
                    }
                    Keyword::Private => {
                        result.set_vis(Visibility::Private);
                    }
                    Keyword::Internal => {
                        result.set_vis(Visibility::AssemblyOnly);
                    }
                    Keyword::Static => {
                        result
                            .specific_mut()
                            .unwrap_class_mut()
                            .insert(ClassImplementationFlags::Static);
                    }
                    Keyword::Partial => {
                        result
                            .specific_mut()
                            .unwrap_class_mut()
                            .insert(ClassImplementationFlags::Partial);
                    }

                    _ => unreachable!(),
                },
                TypeSpecificAttrType::Struct =>  match kw {
                    Keyword::Public => {
                        result.set_vis(Visibility::Public);
                    }
                    Keyword::Private => {
                        result.set_vis(Visibility::Private);
                    }
                    Keyword::Internal => {
                        result.set_vis(Visibility::AssemblyOnly);
                    }
                    Keyword::Ref => {
                        result
                            .specific_mut()
                            .unwrap_struct_mut()
                            .insert(StructImplementationFlags::Ref);
                    }
                    Keyword::Partial => {
                        result
                            .specific_mut()
                            .unwrap_struct_mut()
                            .insert(StructImplementationFlags::Partial);
                    }

                    _ => unreachable!(),
                },
                TypeSpecificAttrType::Interface => match kw {
                    Keyword::Public => {
                        result.set_vis(Visibility::Public);
                    }
                    Keyword::Private => {
                        result.set_vis(Visibility::Private);
                    }
                    Keyword::Internal => {
                        result.set_vis(Visibility::AssemblyOnly);
                    }

                    _ => unreachable!(),
                },
            }
        }

        match index {
            Some(index) => Ok(Self {
                attr: result,
                index,
            }),
            None => Err(ParseError::ExpectLiteral(LiteralKindType::Int)),
        }
    }
}
