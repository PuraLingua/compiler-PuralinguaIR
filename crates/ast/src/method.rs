use std::{collections::HashMap, range::RangeFrom};

use compiler_base::{
    abstract_info::{
        self, IToAbstract, generics::GenericCountRequirement, type_reference::TypeReference,
    },
    global::{
        self,
        attrs::{
            CallConvention, MethodAttr, MethodImplementationFlags, ParameterAttr,
            ParameterImplementationFlags, Visibility,
        },
    },
    utility::uni_parser::{self, ParseError, ParseStream},
};
use enumflags2::BitFlags;
use token::{IntegerToken, Keyword, LiteralKindType, SpecialChar, TokenKind};

use crate::{
    generics::{GenericBound, ParseGenericResult},
    identifier::Identifier,
    item::ItemRef,
};

mod statement;
pub use statement::*;

#[derive(Clone, Debug)]
pub struct MethodReference {
    pub index: ItemRef,
    pub generics: Vec<TypeReference>,
}

impl MethodReference {
    pub fn parse<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
        ty_generics: &[Identifier],
        method_generics: &[Identifier],
    ) -> Result<Self, ParseError<TokenKind>> {
        let index = stream.parse()?;

        let mut generics = Vec::new();
        if stream.require(SpecialChar::BracketOpen).is_ok() {
            while let Ok(g) = stream
                .call(|stream| crate::ty::type_reference(stream, ty_generics, method_generics))
            {
                generics.push(g);
            }
            stream.require(SpecialChar::BracketClose)?;
        }

        Ok(MethodReference { index, generics })
    }
}

#[derive(Clone, Debug)]
pub struct Method {
    pub attr: MethodAttr<TypeReference>,
    pub call_conv: CallConvention,
    pub index: u32,
    pub name: Identifier,
    pub generics: Vec<Identifier>,
    pub is_generic_infinite: bool,
    pub args: Vec<Parameter>,
    pub return_type: TypeReference,
    pub r#impl: Option<(TypeReference, MethodReference)>,
    pub locals: Vec<Identifier>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub generic_bounds: HashMap<String, Vec<GenericBound>>,
    pub statements: Vec<Statement>,
}

impl IToAbstract for Method {
    type Abstract = abstract_info::type_information::method::Method;

    type Err = !;

    fn to_abstract(&self) -> Result<Self::Abstract, Self::Err> {
        Ok(Self::Abstract {
            name: self.name.var.clone(),
            id: self.index,
            attr: self.attr.clone(),
            parameters: self
                .args
                .iter()
                .map(IToAbstract::to_abstract)
                .map(Result::into_ok)
                .collect(),
            return_type: self.return_type.clone(),
        })
    }
}

impl Method {
    pub fn parse<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
        ty_generics: &[Identifier],
    ) -> Result<Self, ParseError<TokenKind>> {
        assert!(stream.consume().is_some_and(|x| x.kind == Keyword::Method));

        stream.expect(SpecialChar::BracketOpen)?;
        let (mut attr, call_convention, index) = stream.call(attr)?;
        stream.expect(SpecialChar::BracketClose)?;

        let name = stream.parse()?;

        let ParseGenericResult {
            generics,
            is_generic_infinite,
        } = stream.parse()?;

        stream.expect(SpecialChar::ParenthesisOpen)?;
        let args = uni_parser::punctuated::punctuated(
            stream,
            false,
            |stream| stream.call(|stream| Parameter::parse(stream, ty_generics, &generics)),
            |stream| stream.require(SpecialChar::Comma),
        )?
        .into_iter()
        .collect();
        stream.expect(SpecialChar::ParenthesisClose)?;

        stream.expect(SpecialChar::ThinArrow)?;
        let return_type =
            stream.call(|stream| crate::ty::type_reference(stream, ty_generics, &generics))?;

        let r#impl = if stream.expect(Keyword::Impl).is_ok() {
            let method_ref =
                stream.call(|stream| MethodReference::parse(stream, ty_generics, &generics))?;
            stream.expect(Keyword::In)?;
            let ty =
                stream.call(|stream| crate::ty::type_reference(stream, ty_generics, &generics))?;
            Some((ty, method_ref))
        } else {
            None
        };

        let generic_bounds = HashMap::new();
        if stream.expect(Keyword::Where).is_ok() {
            todo!("Wheres haven't been implemented")
        }

        stream.expect(SpecialChar::BraceOpen)?;
        let variables =
            stream.call(|stream| parse_locals(stream, &mut attr, ty_generics, &generics))?;
        let exception_table =
            stream.call(|stream| parse_exception_table(stream, ty_generics, &generics))?;
        let mut statements = Vec::new();
        while stream.expect(SpecialChar::BraceClose).is_err() {
            statements
                .push(stream.call(|stream| Statement::parse(stream, ty_generics, &generics))?);
        }

        Ok(Self {
            attr,
            call_conv: call_convention,
            index,
            name,
            generics,
            is_generic_infinite,
            args,
            return_type,
            r#impl,
            generic_bounds,
            locals: variables,
            statements,
            exception_table,
        })
    }
}

impl Method {
    pub fn is_static(&self) -> bool {
        self.attr.is_static()
    }

    pub fn get_generic_count_requirement(&self) -> GenericCountRequirement {
        if self.is_generic_infinite {
            GenericCountRequirement::AtLeast(RangeFrom {
                start: self.generics.len() as u32,
            })
        } else {
            GenericCountRequirement::Exact(self.generics.len() as u32)
        }
    }
}

fn parse_exception_table<'a>(
    stream: &mut ParseStream<'a, TokenKind>,
    ty_generics: &[Identifier],
    method_generics: &[Identifier],
) -> Result<Vec<ExceptionTableEntry>, ParseError<TokenKind>> {
    fn parse_loc<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
    ) -> Result<ExceptionLoc, ParseError<TokenKind>> {
        let is_rel = stream.expect(Keyword::TryRel).is_ok();
        let loc = stream
            .parse::<IntegerToken>()?
            .get_value(stream.source())
            .unwrap();
        Ok(ExceptionLoc { is_rel, loc })
    }
    fn parse_2loc<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
    ) -> Result<(ExceptionLoc, ExceptionLoc), ParseError<TokenKind>> {
        let a = parse_loc(stream)?;
        stream.expect(SpecialChar::DoubledDot)?;
        let b = parse_loc(stream)?;
        Ok((a, b))
    }
    let mut entries = Vec::new();
    if stream.expect(Keyword::ExceptionTable).is_err() {
        return Ok(entries);
    }
    stream.expect(SpecialChar::BraceOpen)?;

    while stream.expect(SpecialChar::BraceClose).is_err() {
        let try_begin = stream
            .parse::<IntegerToken>()?
            .get_value(stream.source())
            .unwrap();
        stream.expect(SpecialChar::DoubledDot)?;
        let try_end = parse_loc(stream)?;

        let exception_type = crate::ty::type_reference(stream, ty_generics, method_generics)?;
        let filter = if stream.expect(Keyword::Where).is_ok() {
            let m = stream
                .call(|stream| MethodReference::parse(stream, ty_generics, method_generics))?;
            stream.expect(Keyword::In)?;
            let ty = stream
                .call(|stream| crate::ty::type_reference(stream, ty_generics, method_generics))?;
            Some((ty, m))
        } else {
            None
        };

        stream.expect(SpecialChar::Arrow)?;

        stream.expect(SpecialChar::BraceOpen)?;
        let mut catch = None;
        let mut finally = None;
        let mut fault = None;
        while stream.expect(SpecialChar::BraceClose).is_err() {
            if stream.expect(Keyword::ExceptionCatch).is_ok() && catch.is_none() {
                catch = Some(parse_2loc(stream)?);
            } else if stream.expect(Keyword::ExceptionFinally).is_ok() && finally.is_none() {
                finally = Some(parse_2loc(stream)?);
            } else if stream.expect(Keyword::ExceptionFault).is_ok() && fault.is_none() {
                fault = Some(parse_2loc(stream)?);
            }
        }

        let Some(catch) = catch else {
            return Err(ParseError::Custom(global::errors::anyhow!(
                "No catch in an exception table"
            )));
        };

        entries.push(ExceptionTableEntry {
            try_begin,
            try_end,
            exception_type,
            filter,
            catch,
            finally,
            fault,
        })
    }

    Ok(entries)
}

fn parse_locals<'a>(
    stream: &mut ParseStream<'a, TokenKind>,
    attr: &mut MethodAttr<TypeReference>,
    ty_generics: &[Identifier],
    method_generics: &[Identifier],
) -> Result<Vec<Identifier>, ParseError<TokenKind>> {
    let mut locals = Vec::new();
    if stream.expect(Keyword::Locals).is_err() {
        return Ok(Vec::new());
    };

    stream.expect(SpecialChar::ParenthesisOpen)?;
    while let Ok(name) = stream.parse() {
        stream.expect(SpecialChar::Colon)?;
        let ty = crate::ty::type_reference(stream, ty_generics, method_generics)?;
        attr.add_local_variable(ty);
        locals.push(name);
    }
    stream.expect(SpecialChar::ParenthesisClose)?;
    Ok(locals)
}

fn attr<'a>(
    stream: &mut ParseStream<'a, TokenKind>,
) -> Result<(MethodAttr<TypeReference>, CallConvention, u32), ParseError<TokenKind>> {
    let mut result = MethodAttr::new(Visibility::Private, BitFlags::empty(), None, Vec::new());
    let mut index: Option<IntegerToken> = None;
    let mut call_convention = CallConvention::PlatformDefault;
    let mut is_override = false;

    while let Ok(token) = stream.require_f(|x| {
        matches!(x.kind, TokenKind::Keyword(a) if a.is_method_modifier()) || x.kind.is_int()
    }) {
        if token.kind.is_int() {
            index = Some(IntegerToken(token));
            continue;
        }

        let TokenKind::Keyword(tk_kw) = token.kind else {
            unreachable!()
        };

        match tk_kw {
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
                    .impl_flags_mut()
                    .insert(MethodImplementationFlags::Static);
            }
            Keyword::HideWhenCapturing => {
                result
                    .impl_flags_mut()
                    .insert(MethodImplementationFlags::HideWhenCapturing);
            }
            Keyword::Override => {
                is_override = true;
                continue;
            }
            Keyword::CallConvention_C => {
                call_convention = CallConvention::CDecl;
            }
            Keyword::CallConvention_CWithVararg => {
                call_convention = CallConvention::CDeclWithVararg;
            }
            Keyword::CallConvention_FASTCALL => {
                call_convention = CallConvention::Fastcall;
            }
            Keyword::CallConvention_STDCALL => {
                call_convention = CallConvention::Stdcall;
            }
            Keyword::CallConvention_SystemV => {
                call_convention = CallConvention::SystemV;
            }
            Keyword::CallConvention_WIN64 => {
                call_convention = CallConvention::Win64;
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

    if is_override {
        *result.overrides_mut() = index;
    }

    match index {
        Some(index) => Ok((result, call_convention, index)),
        None => Err(ParseError::ExpectLiteral(LiteralKindType::Int)),
    }
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub attr: ParameterAttr,
    pub ty: TypeReference,
}

impl IToAbstract for Parameter {
    type Abstract = abstract_info::type_information::method::Parameter;

    type Err = !;

    fn to_abstract(&self) -> Result<Self::Abstract, Self::Err> {
        Ok(Self::Abstract {
            attr: self.attr,
            name: None,
            ty: self.ty.clone(),
        })
    }
}

impl Parameter {
    pub fn parse<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
        ty_generics: &[Identifier],
        method_generics: &[Identifier],
    ) -> Result<Self, ParseError<TokenKind>> {
        stream.require(SpecialChar::BracketOpen)?;
        let mut attr = ParameterAttr::new(BitFlags::empty());
        while let Ok(token) = stream
            .require_f(|x| matches!(x.kind, TokenKind::Keyword(a) if a.is_parameter_modifier()))
        {
            let TokenKind::Keyword(tk_kw) = token.kind else {
                return Err(ParseError::Unexpect(token));
            };
            if !tk_kw.is_parameter_modifier() {
                return Err(ParseError::Unexpect(token));
            }
            match tk_kw {
                Keyword::Ref => {
                    attr.impl_flags_mut()
                        .insert(ParameterImplementationFlags::ByRef);
                }

                _ => unreachable!(),
            }
        }
        stream.require(SpecialChar::BracketClose)?;

        let ty = stream
            .call(|stream| crate::ty::type_reference(stream, ty_generics, method_generics))?;

        Ok(Self { attr, ty })
    }
}

#[derive(Clone, Debug)]
pub struct ExceptionTableEntry {
    pub try_begin: u64,
    pub try_end: ExceptionLoc,
    pub exception_type: TypeReference,

    pub filter: Option<(TypeReference, MethodReference)>,
    pub catch: (ExceptionLoc, ExceptionLoc),
    pub finally: Option<(ExceptionLoc, ExceptionLoc)>,
    pub fault: Option<(ExceptionLoc, ExceptionLoc)>,
}

#[derive(Clone, Copy, Debug)]
pub struct ExceptionLoc {
    pub is_rel: bool,
    pub loc: u64,
}

impl ExceptionLoc {
    #[inline(always)]
    pub const fn resolve(self, begin: u64) -> u64 {
        if self.is_rel {
            self.loc + begin
        } else {
            self.loc
        }
    }
}
