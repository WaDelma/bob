#![recursion_limit = "1024"]

extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{Ident, Field, Ty, Lit, TyParam, Body, StrStyle, Path, PathSegment, PathParameters, AngleBracketedParameterData, Visibility, MetaItem, NestedMetaItem};

fn unwrap_from_option(ty: Ty) -> Ty {
    if let Ty::Path(_, Path{segments, ..}) = ty {
        let &PathSegment{ref ident, ref parameters} = &segments[0];
        if ident == "Option" {
            if let &PathParameters::AngleBracketed(ref a) = parameters {
                return a.types[0].clone();
            }
        }
    }
    panic!("Tried to unwrap optinal that wasn't optional.");
}

fn wrap_into_option(ty: Ty) -> Ty {
    let mut params = AngleBracketedParameterData::default();
    params.types.push(ty);
    Ty::Path(None, PathSegment {
            ident: Ident::new("Option"),
            parameters: PathParameters::AngleBracketed(params),
        }.into())
}


#[proc_macro_derive(Builder, attributes(builder_name, builder_rename, builder_prefix))]
pub fn create_builder(input: TokenStream) -> TokenStream {
    let item = syn::parse_derive_input(&input.to_string()).unwrap();
    let name = &item.ident;
    let vis = &item.vis;
    let (new, build) = item.attrs.iter()
        .filter_map(|a| {
            if let MetaItem::List(ref name, ref value) = a.value {
                if name == "builder_rename" {
                    return Some(value.iter()
                        .filter_map(|v| {
                            if let &NestedMetaItem::MetaItem(MetaItem::NameValue(ref name, ref value)) = v {
                                if name == "new" || name == "build" {
                                    if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                                        return Some((name == "new", Ident::new(&value[..])));
                                    }
                                }
                            }
                            None
                        })
                        .fold((Ident::new("DUMMY"), Ident::new("DUMMY")), |(new, build), (n, v)| {
                            if n {
                                (v, build)
                            } else {
                                (new, v)
                            }
                        }));
                }
            }
            None
        }).next()
        .unwrap_or((Ident::new("new"), Ident::new("build")));
    let builder = item.attrs.iter()
        .filter_map(|a| {
            if let MetaItem::NameValue(ref name, ref value) = a.value {
                if name == "builder_name" {
                    if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                        return Some(Ident::new(&value[..]));
                    }
                }
            }
            None
        }).next()
        .unwrap_or(Ident::new("Builder"));
    let prefix = item.attrs.iter()
        .filter_map(|a| {
            if let MetaItem::NameValue(ref name, ref value) = a.value {
                if name == "builder_prefix" {
                    if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                        return Some(Ident::new(&value[..]));
                    }
                }
            }
            None
        }).next()
        .unwrap_or(Ident::new(""));
    let bmod = Ident::new(format!("_{}", builder.to_string().to_lowercase()));
    let (impl_generics, ty_generics, _) = item.generics.split_for_impl();
    if let Body::Struct(s) = item.body {
        let (normal_fields, optional_fields): (Vec<_>, Vec<_>) = s.fields().iter().partition(|f| {
            if let Ty::Path(_, ref p) = f.ty {
                if let Some(s) = p.segments.get(0) {
                    if s.ident == "Option" {
                        return false;
                    }
                }
            }
            true
        });
        let opt_fields: Vec<_> = optional_fields.iter()
            .enumerate()
            .map(|(i, f)| Field {
                ident: Some(Ident::new(format!("_o{}", i))),
                vis: Visibility::Inherited,
                attrs: vec![],
                ty: f.ty.clone(),
            }).collect();
        let fields: Vec<_> = normal_fields.iter()
            .enumerate()
            .map(|(i, f)| Field {
                ident: Some(Ident::new(format!("_{}", i))),
                vis: Visibility::Inherited,
                attrs: vec![],
                ty: wrap_into_option(f.ty.clone()),
            }).collect();
        let result_fields = normal_fields.iter()
            .map(|f| &f.ident);
        let field_name: Vec<_> = (0..fields.len())
            .map(|i| Ident::new(format!("_{}", i)))
            .collect();
        let result_opt_fields = optional_fields.iter()
            .map(|f| &f.ident);
        let opt_field_name: Vec<_> = (0..opt_fields.len())
            .map(|i| Ident::new(format!("_o{}", i)))
            .collect();

        let builder_ty_params: Vec<_> = (0..fields.len())
            .map(|i| TyParam {
                ident: Ident::new(format!("_T{}", i)),
                attrs: vec![],
                bounds: vec![],
                default: None,
            })
            .collect();

        let mut ext_generics = item.generics.clone();
        ext_generics.ty_params = builder_ty_params.iter()
            .cloned()
            .chain(ext_generics.ty_params)
            .collect();
        let (ext_impl_generics, ext_ty_generics, ext_where_clause) = ext_generics.split_for_impl();

        let mut start_generics = item.generics.clone();
        start_generics.ty_params = (0..fields.len())
            .map(|_| TyParam {
                ident: Ident::new(format!("{}::O", bmod)),
                attrs: vec![],
                bounds: vec![],
                default: None,
            })
            .chain(start_generics.ty_params)
            .collect();
        let (_, start_ty_generics, start_where_clause) = start_generics.split_for_impl();

        let mut end_generics = item.generics.clone();
        end_generics.ty_params = (0..fields.len())
            .map(|_| TyParam {
                ident: Ident::new(format!("{}::I", bmod)),
                attrs: vec![],
                bounds: vec![],
                default: None,
            })
            .chain(end_generics.ty_params)
            .collect();
        let (_, end_ty_generics, _) = end_generics.split_for_impl();

        let mut tks = {
            let fields = &fields;
            let field_name = &field_name;
            let opt_field_name = &opt_field_name;
            let builder_ty_params = &builder_ty_params;
            quote!(
                #vis mod #bmod {
                    pub struct O;
                    pub struct I;
                }

                #[derive(Clone, Debug)]
                #vis struct #builder #ext_ty_generics #ext_where_clause {
                    _marker: ::std::marker::PhantomData<(#(#builder_ty_params),*)>,
                    #(#fields,)*
                    #(#opt_fields),*
                }

                impl #impl_generics #builder #start_ty_generics #start_where_clause {
                    #vis fn #new() -> #builder #start_ty_generics {
                        #builder {
                            _marker: ::std::marker::PhantomData,
                            #(#field_name: None,)*
                            #(#opt_field_name: None),*
                        }
                    }
                }

                impl #impl_generics #builder #end_ty_generics
                    #ext_where_clause
                {
                    #vis fn #build(self) -> #name #ty_generics {
                        #name {
                            #(#result_fields: self.#field_name.unwrap(),)*
                            #(#result_opt_fields: self.#opt_field_name),*
                        }
                    }
                }
            )
        };
        for (i, (field, fname)) in optional_fields.iter().zip(&opt_field_name).enumerate() {
            let mut opt_field_name = opt_field_name.clone();
            opt_field_name.remove(i);
            let ty = unwrap_from_option(field.ty.clone());
            let prefix = field.attrs.iter()
                .filter_map(|a| {
                    if let MetaItem::NameValue(ref name, ref value) = a.value {
                        if name == "builder_prefix" {
                            if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                                return Some(Ident::new(&value[..]));
                            }
                        }
                    }
                    None
                }).next()
                .unwrap_or(prefix.clone());
            let raw_name = Ident::new(&(if let Some(i) = field.ident.clone() {
                format!("{}", i)
            } else {
                format!("{}", i)
            })[..]);
            let name = Ident::new(&format!("{}{}", prefix, raw_name)[..]);
            let field_name = &field_name;
            let field_name2 = field_name; // TODO: This is workaround as `quote!` doesn't support binding variable twice.
            let opt_field_name = &opt_field_name;
            let opt_field_name2 = opt_field_name; // TODO: This is workaround as `quote!` doesn't support binding variable twice.
            let parsed: String = quote!(
                impl #ext_impl_generics #builder #ext_ty_generics #ext_where_clause {
                    #vis fn #name(self, #raw_name: #ty) -> #builder #ext_ty_generics {
                        #builder {
                            _marker: ::std::marker::PhantomData,
                            #(#field_name: self.#field_name2,)*
                            #fname: Some(#raw_name),
                            #(#opt_field_name: self.#opt_field_name2),*
                        }
                    }
                }
            ).parse().unwrap();
            tks.append(&parsed);

        }
        for (i, (field, fname)) in normal_fields.iter().zip(&field_name).enumerate() {
            let mut field_name = field_name.clone();
            field_name.remove(i);
            let field_name = &field_name;
            let field_name2 = field_name; // TODO: This is workaround as `quote!` doesn't support binding variable twice.
            let opt_field_name = &opt_field_name;
            let opt_field_name2 = opt_field_name; // TODO: This is workaround as `quote!` doesn't support binding variable twice.
            let prefix = field.attrs.iter()
                .filter_map(|a| {
                    if let MetaItem::NameValue(ref name, ref value) = a.value {
                        if name == "builder_prefix" {
                            if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                                return Some(Ident::new(&value[..]));
                            }
                        }
                    }
                    None
                }).next()
                .unwrap_or(prefix.clone());
            let raw_name = Ident::new(&(if let Some(i) = field.ident.clone() {
                format!("{}", i)
            } else {
                format!("{}", i)
            })[..]);
            let name = Ident::new(&format!("{}{}", prefix, raw_name)[..]);
            let ty = &field.ty;
            let mut set_generics = item.generics.clone();
            set_generics.ty_params = builder_ty_params.iter()
                .enumerate()
                .map(|(j, t)| {
                    let mut t = t.clone();
                    if i == j {
                        t.ident = Ident::new(format!("{}::O", bmod));
                    }
                    t
                })
                .chain(set_generics.ty_params)
                .collect();
            let (_, set_ty_generics, _) = set_generics.split_for_impl();

            let mut other_generics = item.generics.clone();
            other_generics.ty_params = builder_ty_params.iter()
                .enumerate()
                .filter_map(|(j, t)| if i == j {
                    None
                } else {
                    Some(t.clone())
                })
                .chain(other_generics.ty_params)
                .collect();
            let (other_impl_generics, _, _) = other_generics.split_for_impl();

            let mut after_set_generics = item.generics.clone();
            after_set_generics.ty_params = builder_ty_params.iter()
                .enumerate()
                .map(|(j, t)| {
                    let mut t = t.clone();
                    if i == j {
                        t.ident = Ident::new(format!("{}::I", bmod));
                    }
                    t
                })
                .chain(after_set_generics.ty_params)
                .collect();
            let (_, after_set_ty_generics, _) = after_set_generics.split_for_impl();
            let parsed: String = quote!(
                impl #other_impl_generics #builder #set_ty_generics #ext_where_clause {
                    #vis fn #name(self, #raw_name: #ty) -> #builder #after_set_ty_generics {
                        #builder {
                            _marker: ::std::marker::PhantomData,
                            #fname: Some(#raw_name),
                            #(#field_name: self.#field_name2,)*
                            #(#opt_field_name: self.#opt_field_name2),*
                        }
                    }
                }
            ).parse().unwrap();
            tks.append(&parsed);
        }
        tks.parse().unwrap()
    } else {
        panic!("Only structs supported.");
    }
}
