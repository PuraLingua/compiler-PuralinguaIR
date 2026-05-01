use std::borrow::Cow;

use compiler_base::{
    abstract_info::type_reference::TypeReference,
    global,
    utility::uni_parser::{self, ParseError, ParseStream, proc_macros::Parse},
};
use token::{
    IntegerToken, Keyword, KwCheck_AllZero, KwFailure, KwIf, KwJump_Absolute, KwJump_Backward,
    KwJump_Forward, KwSuccess, LiteralKind, LiteralKindType, SpecialChar, StatementKeyword,
    TokenKind,
};

use crate::{field::FieldReference, identifier::Identifier, method::MethodReference};

#[derive(Parse, Clone, Debug)]
#[t_token_kind(TokenKind)]
pub enum ArrayLen {
    Static(
        #[custom({
            let _0 = stream.parse::<IntegerToken>()?.get_value(stream.source()).unwrap();
        })]
        u64,
    ),
    Dynamic(Identifier),
}

#[derive(Clone, Debug)]
pub enum Statement {
    /// [`Instruction`] LoadTrue, LoadFalse, Load_*, LoadThis
    ///
    /// [`Instruction`]: pura_lingua::global::instruction::Instruction
    Load {
        content: LoadableContent,
        var: Identifier,
    },
    ReadPointerTo {
        ptr: Identifier,
        size: Identifier,
        destination: Identifier,
    },
    WritePointer {
        source: Identifier,
        size: Identifier,
        ptr: Identifier,
    },

    /// [`Instruction`] IsAllZero
    ///
    /// [`Instruction`]: pura_lingua::global::instruction::Instruction
    Check {
        kind: CheckKind,
        to_check: Identifier,
        result: Identifier,
    },

    NewObject {
        ty: TypeReference,
        ctor: MethodReference,
        args: Vec<Identifier>,
        result: Identifier,
    },
    NewArray {
        element_ty: TypeReference,
        len: ArrayLen,
        result: Identifier,
    },

    InstanceCall {
        val: Identifier,
        method: MethodReference,
        args: Vec<Identifier>,
        result: Identifier,
    },
    StaticCall {
        ty: TypeReference,
        method: MethodReference,
        args: Vec<Identifier>,
        result: Identifier,
    },
    InterfaceCall {
        val: Identifier,
        interface: TypeReference,
        method: MethodReference,
        args: Vec<Identifier>,
        result: Identifier,
    },
    NonPurusCall {
        config: Identifier,
        f_pointer: Identifier,
        args: Vec<Identifier>,
        result: Identifier,
    },

    SetField {
        val: Identifier,
        container: Identifier,
        field: FieldReference,
    },
    SetThisField {
        val: Identifier,
        field: FieldReference,
    },
    SetStaticField {
        val: Identifier,
        ty: TypeReference,
        field: FieldReference,
    },

    Throw {
        val: Identifier,
    },

    ReturnVal {
        val: Identifier,
    },

    Jump {
        condition: JumpCondition,
        ty: JumpTargetType,
        val: u64,
    },
    Nop,
}

impl Statement {
    pub fn parse<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
        ty_generics: &[Identifier],
        method_generics: &[Identifier],
    ) -> Result<Self, ParseError<TokenKind>> {
        fn common_args<'a>(
            stream: &mut ParseStream<'a, TokenKind>,
        ) -> Result<Vec<Identifier>, ParseError<TokenKind>> {
            uni_parser::punctuated::punctuated(
                stream,
                false,
                |stream| stream.parse(),
                |stream| stream.require(SpecialChar::Comma),
            )
            .map(|x| x.into_iter().collect())
        }

        let statement_tk =
            stream.require_f(|x| matches!(x.kind, TokenKind::StatementKeyword(_)))?;
        match statement_tk.kind.unwrap_statement_keyword() {
            StatementKeyword::Load => {
                let literal = stream.call(|stream| {
                    LoadableContent::parse(stream, None, ty_generics, method_generics)
                })?;
                stream.require(SpecialChar::ThinArrow)?;
                let var = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::Load {
                    content: literal,
                    var,
                })
            }

            StatementKeyword::ReadPointerTo => {
                let ptr = stream.parse()?;
                let size = stream.parse()?;
                stream.require(SpecialChar::ThinArrow)?;
                let destination = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::ReadPointerTo {
                    ptr,
                    size,
                    destination,
                })
            }
            StatementKeyword::WritePointer => {
                let source = stream.parse()?;
                let size = stream.parse()?;
                stream.require(SpecialChar::ThinArrow)?;
                let ptr = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::WritePointer { source, size, ptr })
            }

            StatementKeyword::Check => {
                let kind = stream.parse()?;
                let to_check = stream.parse()?;
                stream.require(SpecialChar::ThinArrow)?;
                let result = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::Check {
                    kind,
                    to_check,
                    result,
                })
            }

            StatementKeyword::NewObject => {
                let ty = stream.call(|stream| {
                    crate::ty::type_reference(stream, ty_generics, method_generics)
                })?;
                let ctor = stream.call(|stream| {
                    super::MethodReference::parse(stream, ty_generics, method_generics)
                })?;
                stream.require(SpecialChar::ParenthesisOpen)?;
                let args = stream.call(common_args)?;
                stream.require(SpecialChar::ParenthesisClose)?;
                stream.require(SpecialChar::ThinArrow)?;
                let result = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::NewObject {
                    ty,
                    ctor,
                    args,
                    result,
                })
            }
            StatementKeyword::NewArray => {
                let element_ty = stream.call(|stream| {
                    crate::ty::type_reference(stream, ty_generics, method_generics)
                })?;
                let len = stream.parse()?;
                stream.require(SpecialChar::ThinArrow)?;
                let result = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::NewArray {
                    element_ty,
                    len,
                    result,
                })
            }

            StatementKeyword::InstanceCall => {
                let val = stream.parse()?;
                let method = stream.call(|stream| {
                    super::MethodReference::parse(stream, ty_generics, method_generics)
                })?;
                stream.require(SpecialChar::ParenthesisOpen)?;
                let args = stream.call(common_args)?;
                stream.require(SpecialChar::ParenthesisClose)?;
                stream.require(SpecialChar::ThinArrow)?;
                let result = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::InstanceCall {
                    val,
                    method,
                    args,
                    result,
                })
            }
            StatementKeyword::InterfaceCall => {
                let val = stream.parse()?;
                stream.require(Keyword::As)?;
                let interface = stream.call(|stream| {
                    crate::ty::type_reference(stream, ty_generics, method_generics)
                })?;
                let method = stream.call(|stream| {
                    super::MethodReference::parse(stream, ty_generics, method_generics)
                })?;
                stream.require(SpecialChar::ParenthesisOpen)?;
                let args = stream.call(common_args)?;
                stream.require(SpecialChar::ParenthesisClose)?;
                stream.require(SpecialChar::ThinArrow)?;
                let result = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::InterfaceCall {
                    val,
                    interface,
                    method,
                    args,
                    result,
                })
            }
            StatementKeyword::StaticCall => {
                let ty = stream.call(|stream| {
                    crate::ty::type_reference(stream, ty_generics, method_generics)
                })?;
                let method = stream.call(|stream| {
                    super::MethodReference::parse(stream, ty_generics, method_generics)
                })?;
                stream.require(SpecialChar::ParenthesisOpen)?;
                let args = stream.call(common_args)?;
                stream.require(SpecialChar::ParenthesisClose)?;
                stream.require(SpecialChar::ThinArrow)?;
                let result = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::StaticCall {
                    ty,
                    method,
                    args,
                    result,
                })
            }
            StatementKeyword::NonPurusCall => {
                stream.require(SpecialChar::NumberSign)?;
                let config = stream.parse()?;
                let f_pointer = stream.parse()?;
                stream.require(SpecialChar::ParenthesisOpen)?;
                let args = stream.call(common_args)?;
                stream.require(SpecialChar::ParenthesisClose)?;
                stream.require(SpecialChar::ThinArrow)?;
                let result = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::NonPurusCall {
                    config,
                    f_pointer,
                    args,
                    result,
                })
            }

            StatementKeyword::SetField => {
                let val = stream.parse()?;
                stream.require(SpecialChar::ThinArrow)?;
                let container = stream.parse()?;
                stream.require(SpecialChar::Period)?;
                let field = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::SetField {
                    val,
                    container,
                    field,
                })
            }
            StatementKeyword::SetThisField => {
                let val = stream.parse()?;
                stream.require(SpecialChar::ThinArrow)?;
                let field = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::SetThisField { val, field })
            }
            StatementKeyword::SetStaticField => {
                let val = stream.parse()?;
                stream.require(SpecialChar::ThinArrow)?;
                let ty = stream.call(|stream| {
                    crate::ty::type_reference(stream, ty_generics, method_generics)
                })?;
                let field = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::SetStaticField { val, ty, field })
            }

            StatementKeyword::Throw => {
                let val = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::Throw { val })
            }
            StatementKeyword::ReturnVal => {
                let val = stream.parse()?;
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::ReturnVal { val })
            }
            StatementKeyword::Jump => {
                let condition = stream.parse()?;
                let ty = stream.parse()?;
                let val = stream
                    .parse::<IntegerToken>()?
                    .get_value(stream.source())
                    .unwrap();
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::Jump { condition, ty, val })
            }
            StatementKeyword::Nop => {
                stream.require(SpecialChar::Semicolon)?;
                Ok(Statement::Nop)
            }
        }
    }
}

#[derive(Parse, Clone, Debug)]
#[t_token_kind(TokenKind)]
pub enum JumpCondition {
    IfTrue(KwIf, Identifier),
    CheckSuccess(KwSuccess, CheckKind, Identifier),
    CheckFailure(KwFailure, CheckKind, Identifier),
    Unconditional,
}

#[derive(Clone, Debug, Copy, Parse)]
#[t_token_kind(TokenKind)]
pub enum JumpTargetType {
    Absolute(KwJump_Absolute),
    Forward(KwJump_Forward),
    Backward(KwJump_Backward),
}

#[derive(Parse, Clone, Debug, Copy)]
#[t_token_kind(TokenKind)]
pub enum CheckKind {
    AllZero(KwCheck_AllZero),
}

#[derive(Clone, Debug)]
pub enum LoadableContent {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    String(String),
    Char(char),
    ByteString(Vec<u8>),
    Byte(u8),
    True,
    False,
    This,
    Arg(u64),
    Static {
        ty: TypeReference,
        field: FieldReference,
    },
    Field {
        container: Identifier,
        field: FieldReference,
    },
    Size(TypeReference),
}

impl LoadableContent {
    fn parse<'a>(
        stream: &mut ParseStream<'a, TokenKind>,
        is_negative: Option<bool>,
        ty_generics: &[Identifier],
        method_generics: &[Identifier],
    ) -> Result<Self, ParseError<TokenKind>> {
        let tk = *stream.consume().ok_or(ParseError::UnexpectedEOF)?;

        match tk.kind {
            TokenKind::SpecialChar(SpecialChar::Hyphen) => Self::parse(
                stream,
                Some(!is_negative.unwrap_or(false)),
                ty_generics,
                method_generics,
            ),
            TokenKind::Literal { kind, suffix_start } => match kind {
                LiteralKind::Int {
                    base,
                    empty_int: false,
                } => {
                    let value_s = &stream.token_stream().src[tk.span][..(suffix_start as usize)];
                    let ty = &stream.token_stream().src[tk.span][(suffix_start as usize)..];
                    let radix = match base {
                        token::Base::Binary => 2,
                        token::Base::Octal => 8,
                        token::Base::Decimal => 10,
                        token::Base::Hexadecimal => 16,
                    };
                    let start = if radix == 10 { 0 } else { 2 };
                    match ty {
                        "u8" => Ok(LoadableContent::U8(u8::from_str_radix(
                            &value_s[start..],
                            radix,
                        )?)),
                        "u16" => Ok(LoadableContent::U16(u16::from_str_radix(
                            &value_s[start..],
                            radix,
                        )?)),
                        "u32" => Ok(LoadableContent::U32(u32::from_str_radix(
                            &value_s[start..],
                            radix,
                        )?)),
                        "u64" => Ok(LoadableContent::U64(u64::from_str_radix(
                            &value_s[start..],
                            radix,
                        )?)),
                        "usize" => {
                            if std::io::IsTerminal::is_terminal(&std::io::stderr()) {
                                eprintln!(
                                    "You are using usize, whose size depends on current platform"
                                );
                            }
                            match size_of::<usize>() {
                                1 => Ok(LoadableContent::U8(u8::from_str_radix(
                                    &value_s[start..],
                                    radix,
                                )?)),
                                2 => Ok(LoadableContent::U16(u16::from_str_radix(
                                    &value_s[start..],
                                    radix,
                                )?)),
                                4 => Ok(LoadableContent::U32(u32::from_str_radix(
                                    &value_s[start..],
                                    radix,
                                )?)),
                                8 => Ok(LoadableContent::U64(u64::from_str_radix(
                                    &value_s[start..],
                                    radix,
                                )?)),
                                _ => unimplemented!(),
                            }
                        }
                        "i8" => Ok(LoadableContent::I8(if is_negative.is_some_and(|x| x) {
                            -i8::from_str_radix(&value_s[start..], radix)?
                        } else {
                            i8::from_str_radix(&value_s[start..], radix)?
                        })),
                        "i16" => Ok(LoadableContent::I16(if is_negative.is_some_and(|x| x) {
                            -i16::from_str_radix(&value_s[start..], radix)?
                        } else {
                            i16::from_str_radix(&value_s[start..], radix)?
                        })),
                        "i32" => Ok(LoadableContent::I32(if is_negative.is_some_and(|x| x) {
                            -i32::from_str_radix(&value_s[start..], radix)?
                        } else {
                            i32::from_str_radix(&value_s[start..], radix)?
                        })),
                        "i64" => Ok(LoadableContent::I64(if is_negative.is_some_and(|x| x) {
                            -i64::from_str_radix(&value_s[start..], radix)?
                        } else {
                            i64::from_str_radix(&value_s[start..], radix)?
                        })),
                        "isize" => {
                            if std::io::IsTerminal::is_terminal(&std::io::stderr()) {
                                eprintln!(
                                    "You are using isize, whose size depends on current platform"
                                );
                            }
                            match size_of::<isize>() {
                                1 => Ok(LoadableContent::I8(if is_negative.is_some_and(|x| x) {
                                    -i8::from_str_radix(&value_s[start..], radix)?
                                } else {
                                    i8::from_str_radix(&value_s[start..], radix)?
                                })),
                                2 => Ok(LoadableContent::I16(if is_negative.is_some_and(|x| x) {
                                    -i16::from_str_radix(&value_s[start..], radix)?
                                } else {
                                    i16::from_str_radix(&value_s[start..], radix)?
                                })),
                                4 => Ok(LoadableContent::I32(if is_negative.is_some_and(|x| x) {
                                    -i32::from_str_radix(&value_s[start..], radix)?
                                } else {
                                    i32::from_str_radix(&value_s[start..], radix)?
                                })),
                                8 => Ok(LoadableContent::I64(if is_negative.is_some_and(|x| x) {
                                    -i64::from_str_radix(&value_s[start..], radix)?
                                } else {
                                    i64::from_str_radix(&value_s[start..], radix)?
                                })),
                                _ => unimplemented!(),
                            }
                        }
                        _ => Err(ParseError::Custom(global::errors::anyhow!(
                            "Suffix `{ty}` is unknown"
                        ))),
                    }
                }
                LiteralKind::Int {
                    base: _,
                    empty_int: true,
                } => Err(ParseError::ExpectAllLiterals),
                LiteralKind::Float { .. } => todo!(),
                LiteralKind::Char { terminated } => {
                    assert!(terminated);
                    assert!(is_negative.is_none());
                    let s = &stream.token_stream().src[tk.span][1..(suffix_start as usize - 1)];
                    compiler_base::descape::UnescapeExt::to_unescaped(s)?
                        .chars()
                        .next()
                        .ok_or(ParseError::ExpectLiteral(LiteralKindType::Char))
                        .map(LoadableContent::Char)
                }
                LiteralKind::Byte { terminated } => {
                    assert!(terminated);
                    assert!(is_negative.is_none());
                    let s = &stream.token_stream().src[tk.span][1..(suffix_start as usize - 1)];
                    compiler_base::descape::UnescapeExt::to_unescaped(s)?
                        .as_bytes()
                        .first()
                        .copied()
                        .ok_or(ParseError::ExpectLiteral(LiteralKindType::Byte))
                        .map(LoadableContent::Byte)
                }
                LiteralKind::Str { terminated } => {
                    assert!(terminated);
                    assert!(is_negative.is_none());
                    let s = &stream.token_stream().src[tk.span][1..(suffix_start as usize - 1)];
                    match compiler_base::descape::UnescapeExt::to_unescaped(s)? {
                        Cow::Borrowed(x) => Ok(LoadableContent::String(x.to_owned())),
                        Cow::Owned(x) => Ok(LoadableContent::String(x)),
                    }
                }
                LiteralKind::ByteStr { terminated } => {
                    assert!(terminated);
                    assert!(is_negative.is_none());
                    let s = &stream.token_stream().src[tk.span][1..(suffix_start as usize - 1)];
                    match compiler_base::descape::UnescapeExt::to_unescaped(s)? {
                        Cow::Borrowed(x) => {
                            Ok(LoadableContent::ByteString(x.as_bytes().to_owned()))
                        }
                        Cow::Owned(x) => Ok(LoadableContent::ByteString(x.into_bytes())),
                    }
                }
                LiteralKind::RawStr { n_hashes } => {
                    assert!(is_negative.is_none());
                    let prefix_offset = 2 + n_hashes.unwrap_or(0) as usize;
                    let until = suffix_start as usize - 1 - n_hashes.unwrap_or(0) as usize;
                    Ok(LoadableContent::String(
                        stream.token_stream().src[tk.span][prefix_offset..until].to_owned(),
                    ))
                }
                LiteralKind::RawByteStr { n_hashes } => {
                    assert!(is_negative.is_none());
                    let prefix_offset = 2 + n_hashes.unwrap_or(0) as usize;
                    let until = suffix_start as usize - 1 - n_hashes.unwrap_or(0) as usize;
                    Ok(LoadableContent::ByteString(
                        stream.token_stream().src[tk.span].as_bytes()[prefix_offset..until]
                            .to_owned(),
                    ))
                }
            },
            TokenKind::Keyword(Keyword::True) => {
                if is_negative.is_some_and(|x| x) {
                    Ok(LoadableContent::False)
                } else {
                    Ok(LoadableContent::True)
                }
            }
            TokenKind::Keyword(Keyword::False) => {
                if is_negative.is_some_and(|x| x) {
                    Ok(LoadableContent::True)
                } else {
                    Ok(LoadableContent::False)
                }
            }
            TokenKind::Keyword(Keyword::This) => {
                assert!(is_negative.is_none());
                Ok(LoadableContent::This)
            }
            TokenKind::Keyword(Keyword::Load_Arg) => {
                assert!(is_negative.is_none());
                stream.require(SpecialChar::ParenthesisOpen)?;
                let id = stream
                    .parse::<IntegerToken>()?
                    .get_value(stream.source())
                    .unwrap();
                stream.require(SpecialChar::ParenthesisClose)?;
                Ok(LoadableContent::Arg(id))
            }
            TokenKind::Keyword(Keyword::Static) => {
                assert!(is_negative.is_none());
                stream.require(SpecialChar::ParenthesisOpen)?;
                let ty = stream.call(|stream| {
                    crate::ty::type_reference(stream, ty_generics, method_generics)
                })?;
                let field = stream.parse()?;
                stream.require(SpecialChar::ParenthesisClose)?;
                Ok(LoadableContent::Static { ty, field })
            }
            TokenKind::Keyword(Keyword::Load_Field) => {
                assert!(is_negative.is_none());
                stream.require(SpecialChar::ParenthesisOpen)?;
                let container = stream.parse::<Identifier>()?;
                let field = stream.parse()?;
                stream.require(SpecialChar::ParenthesisClose)?;
                Ok(LoadableContent::Field { container, field })
            }
            TokenKind::Keyword(Keyword::Sizeof) => {
                assert!(is_negative.is_none());
                stream.require(SpecialChar::ParenthesisOpen)?;
                let ty = stream.call(|stream| {
                    crate::ty::type_reference(stream, ty_generics, method_generics)
                })?;
                stream.require(SpecialChar::ParenthesisClose)?;
                Ok(LoadableContent::Size(ty))
            }
            _ => {
                stream.set_index(stream.get_index() - 1);
                Err(ParseError::ExpectAllLiterals)
            }
        }
    }
}
