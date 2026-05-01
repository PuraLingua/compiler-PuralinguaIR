use compiler_base::utility::uni_parser::proc_macros::Parse;
use token::{IntegerToken, TokenKind};

use crate::identifier::Identifier;

#[derive(Parse, Clone, Debug)]
#[t_token_kind(TokenKind)]
pub enum ItemRef {
    ByIndex(
        #[custom({
            let _0 = stream.parse::<IntegerToken>()?.get_value(stream.source()).unwrap().try_into().unwrap();
        })]
        u32,
    ),
    ByName(Identifier),
}
