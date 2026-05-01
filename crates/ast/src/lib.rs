#![feature(never_type)]
#![feature(const_trait_impl)]
#![feature(const_convert)]
#![feature(unwrap_infallible)]
#![feature(adt_const_params)]

use compiler_base::utility::uni_parser::{IParse, ParseError, ParseStream};
use token::TokenKind;

use crate::ty::TypeDef;

pub mod field;
pub mod generics;
pub mod identifier;
pub mod item;
pub mod method;
pub mod ty;

pub static COMPILER_ID: &str = include_str!("../../../__LANG_ID");

pub fn parse(source: String, last_index: u32) -> Result<File, ParseError<TokenKind>> {
    let mut stream = ParseStream::<TokenKind>::new(&source);
    match stream.token_stream().get_all_invalid().first() {
        Some(x) => return Err(ParseError::Unexpect(stream.token_stream().tokens[*x])),
        None => (),
    }

    let mut types = Vec::new();

    while !stream.is_eof() {
        let mut ty = TypeDef::parse(&mut stream)?;
        ty.set_index(ty.index() + last_index);
        types.push(ty);
    }

    Ok(File {
        attrs: Vec::new(),
        types,
    })
}

#[derive(Clone, Debug)]
pub struct File {
    pub attrs: Vec<!>, // TODO: implement it
    pub types: Vec<TypeDef>,
}

impl File {
    pub fn new() -> Self {
        Self {
            attrs: Vec::new(),
            types: Vec::new(),
        }
    }

    pub fn merge(&mut self, mut other: Self) {
        self.attrs.append(&mut other.attrs);
        self.types.append(&mut other.types);
    }

    pub fn sort(&mut self) {
        self.types.sort_by_key(|x| x.index());

        for ty in &mut self.types {
            ty.sort_items();
        }
    }
}
