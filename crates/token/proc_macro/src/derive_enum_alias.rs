use proc_macro2::{Span, TokenStream};
use quote::format_ident;
use syn::{Data, DeriveInput, Error, Lit};

pub fn _impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(Error::new(
            Span::call_site(),
            "DeriveEnumAlias could only derive enums",
        ));
    };

    let ty = input
        .attrs
        .iter()
        .find(|x| x.path().get_ident().is_some_and(|x| x == "alias_of"))
        .ok_or(Error::new(Span::call_site(), "alias_of is necessary"))?;
    let ty = ty.parse_args::<syn::Type>()?;

    let derive = input
        .attrs
        .iter()
        .filter(|x| x.path().get_ident().is_some_and(|x| x == "alias_derive"))
        .map(|x| x.meta.require_list().map(|x| x.tokens.clone()))
        .try_collect::<Vec<_>>()?;

    let derive_const = input
        .attrs
        .iter()
        .filter(|x| {
            x.path()
                .get_ident()
                .is_some_and(|x| x == "alias_const_derive")
        })
        .map(|x| x.meta.require_list().map(|x| x.tokens.clone()))
        .try_collect::<Vec<_>>()?;

    let implement = input
        .attrs
        .iter()
        .filter(|x| x.path().get_ident().is_some_and(|x| x == "implement"))
        .map(|x| x.parse_args::<syn::Ident>().unwrap())
        .collect::<Vec<_>>();

    let prefix = input
        .attrs
        .iter()
        .find(|x| x.path().get_ident().is_some_and(|x| x == "alias_prefix"))
        .map(|x| x.meta.require_name_value())
        .transpose()?
        .and_then(|x| {
            let syn::Expr::Lit(p) = &x.value else {
                return None;
            };
            let Lit::Str(s) = &p.lit else {
                return None;
            };
            Some(s.value())
        })
        .unwrap_or(String::new());

    let suffix = input
        .attrs
        .iter()
        .find(|x| x.path().get_ident().is_some_and(|x| x == "alias_suffix"))
        .map(|x| x.meta.require_name_value())
        .transpose()?
        .and_then(|x| {
            let syn::Expr::Lit(p) = &x.value else {
                return None;
            };
            let Lit::Str(s) = &p.lit else {
                return None;
            };
            Some(s.value())
        })
        .unwrap_or(String::new());

    let mut raw_names = Vec::new();
    let mut names = Vec::new();

    for v in &data.variants {
        raw_names.push(&v.ident);
        names.push(format_ident!("{prefix}{}{suffix}", &v.ident));
    }

    let implement = vec![implement; names.len()];
    let derive = vec![derive; names.len()];
    let derive_const = vec![derive_const; names.len()];

    Ok(quote::quote! {
        #(
            #[repr(transparent)]
            #[derive(#(#derive),*)]
            #[derive_const(#(#derive_const),*)]
            #[allow(warnings)]
            pub struct #names(pub #ty);

            #(
                #implement!(#names, #raw_names);
            )*

            impl const ::std::ops::DerefMut for #names {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }

            impl const ::std::ops::Deref for #names {
                type Target = #ty;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )*
    })
}
