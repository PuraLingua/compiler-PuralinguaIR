#![feature(macro_metavar_expr_concat)]
#![feature(iterator_try_collect)]

use proc_macro_utils::macro_definitions::define_derive_macros;

mod derive_enum_alias;

define_derive_macros! {
    DeriveEnumAlias[alias_of, alias_derive, alias_const_derive, alias_prefix, alias_suffix, implement] => derive_enum_alias::_impl;
}
