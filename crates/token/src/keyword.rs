use std::{fmt::Display, marker::ConstParamTy};

use compiler_base::{pura_lingua::global::StrEnum, utility::uni_lexer::TokenInfo};
use konst::const_panic::PanicFmt;
use proc_macro::DeriveEnumAlias;

use crate::TokenKind;

macro keyword_impl_token_info_alias($i:ident, $name:ident) {
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
                    .require($crate::TokenKind::Keyword($crate::Keyword::$name))
                    .map($i)
            }
        }
    };
}

macro st_keyword_impl_token_info_alias($i:ident, $name:ident) {
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
                    .require($crate::TokenKind::StatementKeyword(
                        $crate::StatementKeyword::$name,
                    ))
                    .map($i)
            }
        }
    };
}

#[allow(unused)]
#[derive(ConstParamTy, Debug, Copy, StrEnum, PanicFmt, Hash, DeriveEnumAlias)]
#[derive_const(Clone, PartialEq, Eq)]
#[alias_of(TokenInfo<TokenKind>)]
#[alias_prefix = "StKw"]
#[alias_derive(Copy, Hash)]
#[alias_const_derive(Clone, PartialEq, Eq)]
#[implement(st_keyword_impl_token_info_alias)]
// cSpell:disable-next-line
#[pfmt(crate = konst::const_panic)]
#[global_crate = "compiler_base::pura_lingua::global"]
pub enum StatementKeyword {
    #[str_val("Load")]
    Load,
    #[str_val("ReadPointerTo")]
    ReadPointerTo,
    #[str_val("WritePointer")]
    WritePointer,

    #[str_val("Check")]
    Check,

    #[str_val("NewObject")]
    NewObject,
    #[str_val("NewArray")]
    NewArray,

    #[str_val("InstanceCall")]
    InstanceCall,
    #[str_val("StaticCall")]
    StaticCall,
    #[str_val("InterfaceCall")]
    InterfaceCall,
    #[str_val("NonPurusCall")]
    NonPurusCall,

    #[str_val("SetField")]
    SetField,
    #[str_val("SetThisField")]
    SetThisField,
    #[str_val("SetStaticField")]
    SetStaticField,

    #[str_val("Throw")]
    Throw,
    #[str_val("ReturnVal")]
    ReturnVal,
    #[str_val("Jump")]
    Jump,

    #[str_val("Nop")]
    Nop,
}

impl Display for StatementKeyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl const PartialEq<str> for StatementKeyword {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl<T: [const] AsRef<str>> const PartialEq<T> for StatementKeyword {
    fn eq(&self, other: &T) -> bool {
        self.as_str().eq(other.as_ref())
    }
}

#[allow(unused, nonstandard_style)]
#[derive(ConstParamTy, Debug, Copy, StrEnum, PanicFmt, Hash, DeriveEnumAlias)]
#[derive_const(Clone, PartialEq, Eq)]
#[alias_of(TokenInfo<TokenKind>)]
#[alias_prefix = "Kw"]
#[alias_derive(Copy, Hash)]
#[alias_const_derive(Clone, PartialEq, Eq)]
#[implement(keyword_impl_token_info_alias)]
// cSpell:disable-next-line
#[pfmt(crate = konst::const_panic)]
#[global_crate = "compiler_base::pura_lingua::global"]
pub enum Keyword {
    #[str_val("class")]
    Class,
    #[str_val("struct")]
    Struct,
    #[str_val("interface")]
    Interface,
    #[str_val("method")]
    Method,
    #[str_val("field")]
    Field,

    #[str_val("public")]
    Public,
    #[str_val("private")]
    Private,
    #[str_val("internal")]
    Internal,

    #[str_val("static")]
    Static,

    #[str_val("partial")]
    Partial,

    #[str_val("override")]
    Override,

    #[str_val("hide_when_capturing")]
    HideWhenCapturing,

    #[str_val("C")]
    CallConvention_C,
    #[str_val("C_VARARG")]
    CallConvention_CWithVararg,
    #[str_val("WIN64")]
    CallConvention_WIN64,
    #[str_val("SYSTEM_V")]
    CallConvention_SystemV,
    #[str_val("STDCALL")]
    CallConvention_STDCALL,
    #[str_val("FASTCALL")]
    CallConvention_FASTCALL,

    #[str_val("__MAIN")]
    Main,

    #[str_val("where")]
    Where,

    #[str_val("impl")]
    Impl,

    #[str_val("exception_table")]
    ExceptionTable,

    #[str_val("try_rel")]
    TryRel,
    #[str_val("ExceptionCatch")]
    ExceptionCatch,
    #[str_val("ExceptionFinally")]
    ExceptionFinally,
    #[str_val("ExceptionFault")]
    ExceptionFault,

    #[str_val("in")]
    In,

    #[str_val("as")]
    As,

    #[str_val("ref")]
    Ref,

    #[str_val("if")]
    If,

    #[str_val("true")]
    True,
    #[str_val("false")]
    False,
    #[str_val("this")]
    This,
    #[str_val("success")]
    Success,
    #[str_val("failure")]
    Failure,

    #[str_val("__arg")]
    Load_Arg,
    #[str_val("__field")]
    Load_Field,

    #[str_val("sizeof")]
    Sizeof,

    #[str_val("locals")]
    Locals,

    #[str_val("__absolute")]
    Jump_Absolute,
    #[str_val("__forward")]
    Jump_Forward,
    #[str_val("__backward")]
    Jump_Backward,

    #[str_val("__all_zero")]
    Check_AllZero,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl const PartialEq<str> for Keyword {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl<T: AsRef<str>> PartialEq<T> for Keyword {
    fn eq(&self, other: &T) -> bool {
        self.as_str().eq(other.as_ref())
    }
}

macro def_is($v:vis $source:ident => $checker:ident) {
    $v const fn $checker(self) -> bool {
        let mut has = false;
        konst::iter::for_each! {
            kw in &Self::$source() => {
                has = self.eq(kw);
                if has {
                    break;
                }
            }
        }
        has
    }
}

impl Keyword {
    const fn push_visibility_modifiers<const N: usize, D: konst::drop_flavor::DropFlavor>(
        builder: &mut konst::array::ArrayBuilder<Self, N, D>,
    ) {
        builder.push(Self::Public);
        builder.push(Self::Private);
        builder.push(Self::Internal);
    }
    pub const fn visibility_modifiers() -> [Keyword; 3] {
        let mut builder = konst::array::ArrayBuilder::of_copy();
        Self::push_visibility_modifiers(&mut builder);

        builder.build()
    }

    def_is!(pub visibility_modifiers => is_visibility_modifier);

    pub const fn type_modifiers() -> [Keyword; Keyword::visibility_modifiers().len()] {
        Self::visibility_modifiers()
    }

    def_is!(pub type_modifiers => is_type_modifier);

    pub const fn class_modifiers() -> [Keyword; Keyword::type_modifiers().len() + 2] {
        let mut builder = konst::array::ArrayBuilder::of_copy();
        builder.extend_from_array(Self::type_modifiers());
        builder.push(Self::Static);
        builder.push(Self::Partial);
        builder.build()
    }

    def_is!(pub class_modifiers => is_class_modifier);

    pub const fn struct_modifiers() -> [Keyword; Keyword::type_modifiers().len() + 2] {
        let mut builder = konst::array::ArrayBuilder::of_copy();
        builder.extend_from_array(Self::type_modifiers());
        builder.push(Self::Ref);
        builder.push(Self::Partial);
        builder.build()
    }

    def_is!(pub struct_modifiers => is_struct_modifier);

    pub const fn interface_modifiers() -> [Keyword; Keyword::type_modifiers().len()] {
        let mut builder = konst::array::ArrayBuilder::of_copy();
        builder.extend_from_array(Self::type_modifiers());
        builder.build()
    }

    def_is!(pub interface_modifiers => is_interface_modifier);

    pub const fn field_modifiers() -> [Keyword; Keyword::visibility_modifiers().len() + 1] {
        let mut builder = konst::array::ArrayBuilder::of_copy();
        Self::push_visibility_modifiers(&mut builder);
        builder.push(Self::Static);
        builder.build()
    }

    def_is!(pub field_modifiers => is_field_modifier);

    pub const fn method_modifiers() -> [Keyword; Keyword::visibility_modifiers().len() + 9] {
        let mut builder = konst::array::ArrayBuilder::of_copy();
        Self::push_visibility_modifiers(&mut builder);
        builder.push(Self::Static);
        builder.push(Self::Override);
        builder.push(Self::HideWhenCapturing);
        builder.push(Self::CallConvention_C);
        builder.push(Self::CallConvention_CWithVararg);
        builder.push(Self::CallConvention_FASTCALL);
        builder.push(Self::CallConvention_STDCALL);
        builder.push(Self::CallConvention_SystemV);
        builder.push(Self::CallConvention_WIN64);
        builder.build()
    }

    def_is!(pub method_modifiers => is_method_modifier);

    pub const fn parameter_modifiers() -> [Keyword; 1] {
        let mut builder = konst::array::ArrayBuilder::of_copy();
        builder.push(Keyword::Ref);
        builder.build()
    }

    def_is!(pub parameter_modifiers => is_parameter_modifier);

    pub const fn const_array_contains<const N: usize>(this: [Self; N], m: &str) -> bool {
        let mut has = false;
        konst::iter::for_each! {
            kw in &this => {
                has = konst::eq_str(kw.as_str(), m);
                if has {
                    break;
                }
            }
        }
        has
    }
}

const _: () = {
    macro t($($i:ident),* $(,)?) {$(
        Keyword::$i();
    )*}
    t!(
        visibility_modifiers,
        type_modifiers,
        class_modifiers,
        struct_modifiers,
        interface_modifiers,
        field_modifiers,
        method_modifiers,
        parameter_modifiers,
    );
};
