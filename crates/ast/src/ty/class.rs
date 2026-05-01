use std::{collections::HashMap, ptr::NonNull, range::RangeFrom};

use compiler_base::{
    abstract_info::{
        IToAbstract, generics::GenericCountRequirement, type_information::TypeInformation,
        type_reference::TypeReference,
    },
    global::attrs::{TypeAttr, TypeSpecificAttrType},
    utility::uni_parser::{ParseError, proc_macros::Parse},
};
use token::{
    BracketCloseChar, BracketOpenChar, IntegerToken, Keyword, KwClass, SpecialChar, TokenKind,
};

use crate::{
    COMPILER_ID,
    field::Field,
    generics::{GenericBound, ParseGenericResult},
    identifier::Identifier,
    method::Method,
    ty::TypeAttrAndIndex,
};

#[derive(Parse, Clone, Debug)]
#[t_token_kind(TokenKind)]
pub struct ClassDef {
    pub kw_class: KwClass,

    pub ch_bracket_open: BracketOpenChar,
    #[custom({
        let TypeAttrAndIndex { attr, index } = stream.parse::<TypeAttrAndIndex<{ TypeSpecificAttrType::Class }>>()?;
        let index = index.get_value(stream.source()).unwrap().try_into().unwrap();
    })]
    pub attr: TypeAttr,
    #[do_not_generate]
    pub index: u32,
    pub ch_bracket_close: BracketCloseChar,

    #[custom({
        let main = if stream.expect(Keyword::Main).is_ok() {
            stream.expect(SpecialChar::ParenthesisOpen)?;
            let id = stream.parse::<IntegerToken>()?.get_value(stream.source()).unwrap().try_into().unwrap();
            stream.expect(SpecialChar::ParenthesisClose)?;
            Some(id)
        } else {
            None
        };
    })]
    pub main: Option<u32>,

    pub name: Identifier,

    #[custom({
        let ParseGenericResult { generics, is_generic_infinite } = stream.parse()?;
    })]
    pub generics: Vec<Identifier>,
    #[do_not_generate]
    pub is_generic_infinite: bool,

    #[custom({
        let parent = if stream.expect(SpecialChar::Colon).is_ok() {
            Some(stream.call(|stream| crate::ty::type_reference(stream, &generics, &[]))?)
        } else {
            None
        };
    })]
    pub parent: Option<TypeReference>,

    #[custom({
        let generic_bounds = HashMap::new();
        if stream.expect(Keyword::Where).is_ok() {
            todo!("Wheres haven't been implemented")
        }
    })]
    pub generic_bounds: HashMap<Identifier, Vec<GenericBound>>,

    #[custom({
        let mut methods = Vec::new();
        let mut fields = Vec::new();

        stream.expect(SpecialChar::BraceOpen)?;
        while stream.require(SpecialChar::BraceClose).is_err()
            && let Some(&current) = stream.current()
        {
            match current.kind {
                TokenKind::Keyword(Keyword::Method) => {
                    methods.push(stream.call(|stream| Method::parse(stream, &generics))?);
                }
                TokenKind::Keyword(Keyword::Field) => {
                    fields.push(stream.call(|stream| Field::parse(stream, &generics))?);
                }

                _ => return Err(ParseError::Unexpect(current)),
            }
        }
    })]
    pub methods: Vec<Method>,
    #[do_not_generate]
    pub fields: Vec<Field>,
}

impl IToAbstract for ClassDef {
    type Abstract = TypeInformation;

    type Err = !;

    fn to_abstract(&self) -> Result<Self::Abstract, Self::Err> {
        let generic_count = self.get_generic_count_requirement();
        Ok(TypeInformation {
            assembly_name: String::new(),
            name: {
                let mut raw_name = self.name.var.clone();
                generic_count.decorate(&mut raw_name);
                raw_name
            },
            id: self.index,
            attr: self.attr,
            generic_count,

            parent: self.parent.clone(),

            implemented_interfaces: self.collect_implemented_interfaces(),

            fields: self
                .fields
                .iter()
                .map(IToAbstract::to_abstract)
                .map(Result::into_ok)
                .collect(),
            methods: self
                .methods
                .iter()
                .map(IToAbstract::to_abstract)
                .map(Result::into_ok)
                .collect(),
            provider_name: NonNull::from_ref(COMPILER_ID),
            additional_data: None,
        })
    }
}

impl ClassDef {
    pub fn sort_items(&mut self) {
        self.methods.sort_by_key(|x| x.index);
        self.fields.sort_by_key(|x| x.index);
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

    pub fn collect_implemented_interfaces(&self) -> Vec<TypeReference> {
        let mut result = Vec::new();
        for method in &self.methods {
            if let Some((ty, _)) = &method.r#impl {
                if !result.contains(ty) {
                    result.push(ty.clone());
                }
            }
        }

        result
    }
}
