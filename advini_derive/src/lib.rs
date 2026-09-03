use std::str::FromStr;

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    Data, DataEnum, DeriveInput, Expr, Fields, GenericParam, Generics, Ident, LitInt, LitStr, Meta, Type, parse_macro_input, parse_quote};

#[proc_macro_derive(Ini)]
pub fn derive_ini(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let name = input.ident;
    let Data::Enum(data) = input.data else {
        unimplemented!()
    };
    let matched = match_quote(&data, &name);
    let self_matched = match_self(&data, name.clone());
    let res = quote!(
		impl #impl_generics Ini<'_> for #name #ty_generics #where_clause {
			fn eat<'a>(__input: &'a str, _additional: Self::Arg) -> Result<(&'a str, Self), IniParseError> {
				#matched
			}
			fn vomit(&self, _additional: Self::Arg) -> String {
				#self_matched
			}
		}
	).into();
    res
}
fn match_self(data: &DataEnum, name: Ident) -> TokenStream {
    let mut variants = vec![];
    for variant in data.variants.clone() {
        let variant_ident = variant.ident.clone();
        let variant_ident_string = variant.ident.clone().to_string();
        let field_idents = (0..(variant.fields.len()))
            .map(|i| Ident::new(&format!("field_{i}"), Span::call_site()))
            .collect::<Vec<_>>();
        let braces = if variant.fields.is_empty() {
            quote!()
        } else {
            quote!((#(#field_idents),*))
        };
        variants.push(quote!(
			#name::#variant_ident #braces => {
				let mut res = [#variant_ident_string.to_string(), #(#field_idents.vomit(_additional)), *].join(",");
				res.push_str(",");
				res
			},
		));
    }
    quote!(
        match self {
            #(#variants)*
        }
    )
}
fn match_quote(data: &DataEnum, ident: &Ident) -> TokenStream {
    let mut variants = vec![];
    for variant in data.variants.clone() {
        let variant_ident = variant.ident.clone();
        let variant_ident_string = variant.ident.clone().to_string();
        let mut fields = vec![];
        let field_idents = (0..(variant.fields.len()))
            .map(|i| Ident::new(&format!("field_{i}"), Span::call_site()))
            .collect::<Vec<_>>();
        for (i, field) in variant.fields.iter().enumerate() {
            let ty = field.ty.clone();
            let field_name = Ident::new(&format!("field_{i}"), Span::call_site());
            fields.push(quote!(
                let (__input, #field_name) = <#ty as advini::Ini>::eat(__input, _additional)?;
            ));
        }
        let braces = if variant.fields.is_empty() {
            quote!()
        } else {
            quote! { (#(#field_idents),*) }
        };
        variants.push(quote!(
            #variant_ident_string => {
                #(#fields)*
                Ok((__input, #ident::#variant_ident #braces))
            },
        ));
    }
    quote!(
        let (__input, res) = <String as advini::Ini>::eat(__input, _additional)?;
        match __input {
            #(#variants)*
            _ => { Err(advini::IniParseError::Error("Wrong variant!".into())) }
        }
    )
}
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
		impl #impl_generics Sections<'_> for #name #ty_generics #where_clause {
			type Arg = #additional;
			fn from_section(mut prop: advini::Section, _additional: Self::Arg) -> Result<(Self, indexmap::IndexMap<String, String>), advini::SectionError> {
				#from
			}
			fn to_section(&self, _additional: Self::Arg) -> advini::Section {
				#into
			}
		}
	).into()
}
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(advini::Ini));
            // type_param.bounds.push(parse_quote!(PartialEq));
        }
    }
    generics
}
struct FieldInfo {
    pub aliases: (Ident, Vec<LitStr>),
    pub used: bool,
    pub default: Option<syn::Expr>,
    pub additional: Option<usize>, //Vec<syn::Expr>,
    pub inline: bool,
    pub ty: syn::Type,
}
fn to_litstr(ident: &Ident) -> syn::LitStr {
    syn::LitStr::new(&ident.to_string(), ident.span())
}

fn trait_body(data: &Data, ident: &Ident) -> (TokenStream, TokenStream, TokenStream) {
    let mut fields = vec![];
    let mut additional_types = vec![];
	if let Data::Struct(s) = data {
		if let Fields::Named(f) = &s.fields {
            for field in &f.named {
                fields.push(FieldInfo {
                    aliases: (field.ident.clone().unwrap(), Vec::new()),
                    used: true,
                    additional: None,
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
                                        .push(to_litstr(&meta.path.get_ident().clone().unwrap()));
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
                                    .push(to_litstr(&p.get_ident().clone().unwrap()));
                            }
							syn::Meta::NameValue(meta) => {
								if let syn::Expr::Lit(exprlit) = &meta.value {
									if let syn::Lit::Str(litstr) = &exprlit.lit {
										fields
											.last_mut()
											.unwrap()
											.aliases
											.1
											.push(litstr.clone());
									}
								}
							},
                            _ => {
								unimplemented!()
							},
                        }
                    }
                    if attr.path().is_ident("additional") {
						if let Meta::NameValue(meta) = &attr.meta {
                            if let syn::Expr::Lit(exprlit) = &meta.value {
                                if let syn::Lit::Str(litstr) = &exprlit.lit {
                                    let ty = litstr.parse::<Type>().unwrap();
									if !additional_types.contains(&ty) {
										additional_types.push(ty.clone());
                                    };
									let pos = additional_types.iter().position(|x| x==&ty);
                                    fields.last_mut().unwrap().additional = pos;
                                }
                            }
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
        } else if let Fields::Unit = &s.fields {
            quote!(
                #ident
            );
        } else {
			{}
        }
	} else {
		unimplemented!()
	}

    let field_declarations = fields.iter().map(|field| {
        let ident = &field.aliases.0;
        let ty = &field.ty;
		let idents = field.aliases.1.clone();//.iter().map(|x| to_litstr(&x)).collect::<Vec<_>>();
		let name = to_litstr(&field.aliases.0);
		let additional = if let Some(additional) = field.additional {
			let add = LitInt::new(&additional.to_string(), Span::call_site());
			quote! { _additional.#add }
		} else {
			quote! { Default::default() }
		};
		let default = if let Some(default) = &field.default {
            quote! {
				.unwrap_or(("", #default.into()))
			}
        } else {
			quote!(?)
		};
		
		if field.inline && field.used {
			quote!()
		} else if !field.used {
			let default = &field.default.clone().unwrap();
			quote! {
				let #ident = #default;
			}
		} else {
			quote! {
				let mut #ident;
				(_, #ident) = <#ty as advini::Ini>::eat(
					&*[#name, #(#idents), *]
					.iter()
					.filter_map(|name| prop.shift_remove(&**name))
					.next()
					.unwrap_or_else(|| {
						//dbg!(#name, #(#idents), *);
						String::new()
					}),
					#additional).map_err(|err| err.to_string())
					#default;
			}
		}
    });
    let name = &ident;
    let construct_fields = fields.iter().map(|f| {
        let ident = &f.aliases.0;
        if let Some(_) = &f.default {
            quote!(#ident)
        } else {
            quote!(#ident)//: #ident.unwrap())
        }
    });
    let struct_construction = quote! {
        #name {
            #(#construct_fields),*
        }
        //#name::new( #(#construct_fields),* )
    };
    let inlined_fields = fields.iter().filter(|f| f.inline && f.used).map(|f| {
        let ident = &f.aliases.0;
		let additional = if let Some(additional) = f.additional {
			let add = LitInt::new(&additional.to_string(), Span::call_site());
			quote! { &_additional.#add }
		} else {
			quote! { Default::default() }
		};
        let ty = &f.ty;
        quote!(
            let res = <#ty as advini::Sections>::from_section(remaining, #additional)?;
			remaining = res.1;
			let #ident = res.0.into();
        )
    });
    let field_declarations = quote! {
        #(#field_declarations )*
        let mut remaining = prop;
        #(#inlined_fields)*
        Ok((#struct_construction, remaining))
    };

    let to_section = to_section_body(&fields, ident);
    let additional = if additional_types.is_empty() {
        TokenStream::from_str("()").unwrap()
    } else {
        quote!(
            ((), #(#additional_types,)*)
        )
    };
    (field_declarations, to_section, additional)
}

fn to_section_body(fields: &Vec<FieldInfo>, _ident: &Ident) -> TokenStream {
    let fields_filtered = fields.iter().filter(|f| f.used);
    let fields_declarations = fields_filtered.clone().map(|f| {
        let name = to_litstr(&f.aliases.0);
        let ident = &f.aliases.0;
		let additional = if let Some(additional) = f.additional {
			let add = LitInt::new(&additional.to_string(), Span::call_site());
			quote! { &_additional.#add }
		} else {
			quote! { Default::default() }
		};
        let ty = &f.ty;
        if let Some(default) = &f.default {
            if f.inline {
                quote_spanned!( Span::call_site() => {
                    if self.#ident != #default {
                        section.extend(<#ty as advini::Sections>::to_section(&self.#ident, #additional));
                    }
                })
            } else {
                quote_spanned!( Span::call_site() => {
                    if self.#ident != #default {
                        section.insert(#name.to_string(), self.#ident.vomit(#additional));
                    }
                })
            }
        } else {
            if f.inline {
                quote_spanned!( Span::call_site() => {
                    section.extend(<#ty as advini::Sections>::to_section(&self.#ident, #additional));
                })
            } else {
                quote!(
                    section.insert(#name.to_string(), self.#ident.vomit(#additional));
                )
            }
        }
    });
    quote!(
        let mut section = indexmap::IndexMap::new();
        #(
            #fields_declarations
        )*
        section
    )
}
