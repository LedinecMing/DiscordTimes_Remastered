use std::str::FromStr;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, Parser},
    parse_macro_input, parse_quote, Data, DeriveInput, Expr, Fields, GenericParam, Generics, Ident,
    Index, Meta, MetaNameValue,
};

#[proc_macro_derive(
    Sections,
    attributes(alias, unused, default_value, inline_parsing, additional)
)]
pub fn derive_sections(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let name = input.ident;

    let (from, into, additional) = trait_body(&input.data, &name);
    quote!(
		impl #impl_generics Sections for #name #ty_generics #where_clause {
			fn from_section(sec: Section) -> Result<(Self, std::collections::HashMap<String, String>), SectionError> {
				let prop = sec;
				#from
			}
			fn to_section(&self) -> Section {
				#into
			}
		}
	).into()
}
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(Ini));
        }
    }
    generics
}
struct FieldInfo {
    pub aliases: (Ident, Vec<Ident>),
    pub used: bool,
    pub default: Option<syn::Expr>,
    pub additional: Vec<syn::Expr>,
    pub inline: bool,
    pub ty: syn::Type,
}
fn to_litstr(ident: &Ident) -> syn::LitStr {
    syn::LitStr::new(&ident.to_string(), ident.span())
}

fn trait_body(data: &Data, ident: &Ident) -> (TokenStream, TokenStream, TokenStream) {
    let mut fields = Vec::new();
    let mut additional_types = Vec::new();
    match data {
        Data::Enum(_) => {
            unimplemented!()
        }
        Data::Struct(s) => {
            match &s.fields {
                Fields::Named(f) => {
                    for field in &f.named {
                        fields.push(FieldInfo {
                            aliases: (field.ident.clone().unwrap(), Vec::new()),
                            used: true,
                            additional: Vec::new(),
                            default: None,
                            inline: false,
                            ty: field.ty.clone(),
                        });
                        for attr in &field.attrs {
                            if attr.path().is_ident("alias") {
                                match &attr.meta {
                                    syn::Meta::List(ml) => {
                                        ml.parse_nested_meta(|meta| {
                                            fields
                                                .last_mut()
                                                .unwrap()
                                                .aliases
                                                .1
                                                .push(meta.path.get_ident().unwrap().clone());
                                            Ok(())
                                        })
                                        .ok();
                                    }
                                    syn::Meta::Path(p) => {
                                        fields
                                            .last_mut()
                                            .unwrap()
                                            .aliases
                                            .1
                                            .push(p.get_ident().unwrap().clone());
                                    }
                                    _ => unimplemented!(),
                                }
                            }
                            if attr.path().is_ident("additional") {
                                match &attr.meta {
                                    syn::Meta::List(ml) => {
                                        ml.parse_nested_meta(|meta| {
                                            let a = <syn::Expr as Parse>::parse
                                                .parse(meta.path.into_token_stream().into())
                                                .unwrap();
                                            if !additional_types.contains(&a) {
                                                additional_types.push(a.clone());
                                            };
                                            fields.last_mut().unwrap().additional.push(a);
                                            Ok(())
                                        })
                                        .ok();
                                    }
                                    _ => unimplemented!(),
                                }
                            }
                            if attr.path().is_ident("unused") {
                                fields.last_mut().unwrap().used = false;
                            }
                            if attr.path().is_ident("inline_parsing") {
                                fields.last_mut().unwrap().inline = true;
                            }
                            if attr.path().is_ident("default_value") {
                                //fields.last_mut().unwrap().default = Some(syn::LitStr::attr.meta.require_name_value().unwrap().value.clone());
                                if let Meta::NameValue(meta) = &attr.meta {
                                    if let syn::Expr::Lit(exprlit) = &meta.value {
                                        if let syn::Lit::Str(litstr) = &exprlit.lit {
                                            fields.last_mut().unwrap().default =
                                                Some(litstr.parse::<Expr>().unwrap());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Fields::Unit => {
                    quote!(
                        #ident
                    );
                }
                Fields::Unnamed(_) => {}
            }
        }
        Data::Union(_) => unimplemented!(),
    }

    let field_declarations = fields.iter().map(|field| {
        let ident = &field.aliases.0;
        let ty = &field.ty;
        if let Some(default) = &field.default {
            quote! {
                let mut #ident : #ty = #default.into();
            }
        } else {
            quote! {
                let mut #ident : Option<#ty> = None;
            }
        }
    });
    let match_patterns = fields.iter().filter(|f| f.used).map(|field| {
        let ident = &field.aliases.0;
        let ty = &field.ty;
        let idents = field.aliases.1.iter().map(to_litstr);
        let name = to_litstr(&ident);
        let pattern = if field.aliases.1.is_empty() {
            quote!( #name )
        } else {
            quote!( #name | #(#idents)|* )
        };
        let res = if let Some(_) = &field.default {
            quote!(res)
        } else {
            quote!(Some(res))
        };
        let mut ty_ = quote!( #ty );
        let tystr = ty.to_token_stream().to_string();
        if tystr.starts_with("Option") {
            let tystr = syn::LitStr::new(
                tystr.split_once("<").unwrap().1.rsplit_once(">").unwrap().0,
                Span::call_site(),
            )
            .parse::<syn::Type>()
            .unwrap();
            ty_ = quote!( #tystr );
        }
        if field.inline {
            quote!()
        } else {
            quote!(
               #pattern => #ident = match <#ty_ as Ini>::eat(v) {
                   Ok((res, chars)) => {
                       v = chars;
                       #res.into()
                   },
                   Err(IniParseError::Empty(chars)) => {
                       v = chars;
                       continue;
                   },
                   Err(IniParseError::Error(info)) => {
                       println!("{}", info);
                       continue;
                   }
               },
            )
        }
    });
    let name = &ident;
    let construct_fields = fields.iter().filter(|f| f.used).map(|f| {
        let ident = &f.aliases.0;
        if let Some(_) = &f.default {
            quote!(#ident)
        } else {
            quote!(#ident.unwrap())
        }
    });
    let struct_construction = quote! {
        #name::new( #(#construct_fields),* )
    };
    dbg!(&struct_construction.to_string());
    let inlined_fields = fields.iter().filter(|f| f.inline).map(|f| {
        let ident = &f.aliases.0;
        let ty = &f.ty;
        quote!(
            let res = <#ty as Sections>::from_section(remaining).unwrap();
            (#ident, remaining) = (res.0.into(), res.1);
        )
    });

    let field_declarations = quote! {
        #(#field_declarations )*
        let mut remaining = std::collections::HashMap::new();
        for (k, value) in prop.iter() {
            let mut v = value.chars();
            match &**k {
                #(#match_patterns)*
                any => {
                    remaining.insert(k.clone(), value.clone());
                }
            }
        };
        #(#inlined_fields)*
        Ok((#struct_construction, remaining))
    };

    let to_section = to_section_body(&fields, ident);
    let additional = if additional_types.is_empty() {
        TokenStream::from_str("()").unwrap()
    } else {
        quote!(
            (#(#additional_types,)*)
        )
    };
    (field_declarations, to_section, additional)
}

fn to_section_body(fields: &Vec<FieldInfo>, _ident: &Ident) -> TokenStream {
    let fields_filtered = fields.iter().filter(|f| f.used);
    let fields_declarations = fields_filtered.clone().map(|f| {
        let name = to_litstr(&f.aliases.0);
        let ident = &f.aliases.0;
        let ty = &f.ty;
        if f.ty.to_token_stream().to_string().starts_with("Option") {
            quote_spanned!( Span::call_site() =>
                if let Some(val) = &self.#ident {
                    section.insert(#name.into(), val.vomit());
                }
            )
        } else if f.inline {
            quote_spanned!( Span::call_site() =>
                section.extend(<#ty as Sections>::to_section(&self.#ident));
            )
        } else {
            quote!(
                section.insert(#name.into(), self.#ident.vomit());
            )
        }
    });
    quote!(
        let mut section = std::collections::HashMap::new();
        #(
            #fields_declarations
        )*
        section
    )
}
