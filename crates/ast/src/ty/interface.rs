use std::collections::HashMap;
use std::ptr::NonNull;
use std::range::RangeFrom;

use compiler_base::abstract_info::IToAbstract;
use compiler_base::abstract_info::generics::GenericCountRequirement;
use compiler_base::abstract_info::type_information::TypeInformation;
use compiler_base::global::attrs::{TypeAttr, TypeSpecificAttrType};
use compiler_base::utility::uni_parser::{self, ParseError};
use compiler_base::{
    abstract_info::type_reference::TypeReference, utility::uni_parser::proc_macros::Parse,
};
use token::{BracketCloseChar, BracketOpenChar, Keyword, KwInterface, SpecialChar, TokenKind};

use crate::COMPILER_ID;
use crate::generics::ParseGenericResult;
use crate::ty::TypeAttrAndIndex;
use crate::{generics::GenericBound, identifier::Identifier, method::Method};

#[derive(Clone, Debug, Parse)]
#[t_token_kind(TokenKind)]
pub struct InterfaceDef {
    pub kw_interface: KwInterface,

    pub ch_bracket_open: BracketOpenChar,
    #[custom({
		let TypeAttrAndIndex { attr, index } = stream.parse::<TypeAttrAndIndex<{ TypeSpecificAttrType::Interface }>>()?;
        let index = index.get_value(stream.source()).unwrap().try_into().unwrap();
	})]
    pub attr: TypeAttr,
    #[do_not_generate]
    pub index: u32,
    pub ch_bracket_close: BracketCloseChar,

    pub name: Identifier,

    #[custom({
		let ParseGenericResult { generics, is_generic_infinite } = stream.parse()?;
	})]
    pub generics: Vec<Identifier>,
    #[do_not_generate]
    pub is_generic_infinite: bool,

    #[custom({
		let required_interfaces = if stream.expect(SpecialChar::Colon).is_ok() {
			stream.call(|stream| uni_parser::punctuated::punctuated(
				stream,
				true,
				|stream| stream.call(|stream| crate::ty::type_reference(stream, &generics, &[])),
				|stream| stream.require(SpecialChar::Comma),
			))?
			.into_iter()
			.collect()
		} else {
			vec![]
		};
	})]
    pub required_interfaces: Vec<TypeReference>,

    #[custom({
        let generic_bounds = HashMap::new();
        if stream.expect(Keyword::Where).is_ok() {
            todo!("Wheres haven't been implemented")
        }
    })]
    pub generic_bounds: HashMap<Identifier, Vec<GenericBound>>,

    #[custom({
        let mut methods = Vec::new();

        stream.expect(SpecialChar::BraceOpen)?;
        while stream.require(SpecialChar::BraceClose).is_err()
            && let Some(&current) = stream.current()
        {
            match current.kind {
                TokenKind::Keyword(Keyword::Method) => {
                    methods.push(stream.call(|stream| Method::parse(stream, &generics))?);
                }

                _ => return Err(ParseError::Unexpect(current)),
            }
        }
    })]
    pub methods: Vec<Method>,
}

impl IToAbstract for InterfaceDef {
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

            parent: None,

            implemented_interfaces: self.required_interfaces.clone(),

            fields: Vec::new(),
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

impl InterfaceDef {
    pub fn sort_items(&mut self) {
        self.methods.sort_by_key(|x| x.index);
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
