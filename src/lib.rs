#![recursion_limit = "1024"]

extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{Ident, Field, Ty, Lit, Generics, TyParam, Body, StrStyle, Attribute, Path, PathSegment, PathParameters, AngleBracketedParameterData, Visibility, MetaItem, NestedMetaItem};

use std::mem::swap;

#[proc_macro_derive(Builder, attributes(builder_name, builder_rename, builder_prefix))]
pub fn create_builder(input: TokenStream) -> TokenStream {
    let item = syn::parse_derive_input(&input.to_string()).unwrap();
    if let Body::Struct(s) = item.body {
        let builder = get_builder_name(&item.attrs);
        let (new, build) = get_builder_methods(&item.attrs);
        let prefix = get_setter_prefix(&item.attrs, Ident::new(""));

        let name = &item.ident;
        let vis = &item.vis;
        let bmod = Ident::new(format!("_{}", builder.to_string().to_lowercase()));
        let (impl_generics, ty_generics, _) = item.generics.split_for_impl();
        let (opt_res_fields, res_fields): (Vec<_>, Vec<_>)
            = s.fields().iter().partition(|f| is_option(&f.ty));

        let opt_build_fields: Vec<_> = opt_res_fields.iter()
            .enumerate()
            .map(|(i, f)|
                priv_field(format!("_o{}", i), f.ty.clone()))
            .collect();

        let build_fields: Vec<_> = res_fields.iter()
            .enumerate()
            .map(|(i, f)|
                priv_field(format!("_{}", i), wrap_into_option(f.ty.clone())))
            .collect();

        let result_fields = res_fields.iter().map(|f| &f.ident);
        let field_name: Vec<_> = (0..build_fields.len())
            .map(|i| Ident::new(format!("_{}", i)))
            .collect();

        let result_opt_fields = opt_res_fields.iter().map(|f| &f.ident);
        let opt_field_name: Vec<_> = (0..opt_build_fields.len())
            .map(|i| Ident::new(format!("_o{}", i)))
            .collect();

        let builder_ty_params: Vec<_> = (0..build_fields.len())
            .map(|i| plain_ty_param(format!("_T{}", i)))
            .collect();

        let mut ext_generics = item.generics.clone();
        add_ty_params(&mut ext_generics, builder_ty_params.clone());
        let (ext_impl_generics, ext_ty_generics, ext_where_clause) = ext_generics.split_for_impl();

        let mut start_generics = item.generics.clone();
        add_ty_params(&mut start_generics,
            (0..build_fields.len())
                .map(|_| plain_ty_param(format!("{}::O", bmod))));
        let (_, start_ty_generics, start_where_clause) = start_generics.split_for_impl();

        let mut end_generics = item.generics.clone();
        add_ty_params(&mut end_generics,
            (0..build_fields.len())
                .map(|_| plain_ty_param(format!("{}::I", bmod))));
        let (_, end_ty_generics, _) = end_generics.split_for_impl();

        let mut tks = {
            let build_fields = &build_fields;
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
                    #(#build_fields,)*
                    #(#opt_build_fields),*
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
        for (i, (field, fname)) in opt_res_fields.iter().zip(&opt_field_name).enumerate() {
            let mut opt_field_name = opt_field_name.clone();
            opt_field_name.remove(i);
            let (field_name, field_name2) = (&field_name, &field_name);
            let (opt_field_name, opt_field_name2) = (&opt_field_name, &opt_field_name);

            let ty = unwrap_from_option(&field.ty);
            let prefix = get_setter_prefix(&field.attrs, prefix.clone());
            let raw_name = field.ident.clone().unwrap_or_else(|| i.to_string().into());
            let name = Ident::new(&format!("{}{}", prefix, raw_name)[..]);

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
        for (i, (field, fname)) in res_fields.iter().zip(&field_name).enumerate() {
            let mut field_name = field_name.clone();
            field_name.remove(i);
            let (field_name, field_name2) = (&field_name, &field_name);
            let (opt_field_name, opt_field_name2) = (&opt_field_name, &opt_field_name);

            let ty = &field.ty;
            let prefix = get_setter_prefix(&field.attrs, prefix.clone());
            let raw_name = field.ident.clone().unwrap_or_else(|| i.to_string().into());
            let name = Ident::new(&format!("{}{}", prefix, raw_name)[..]);

            let mut other_generics = item.generics.clone();
            add_ty_params(&mut other_generics, builder_ty_params
                .iter().enumerate()
                .filter_map(|(j, t)| if i == j {
                    None
                } else {
                    Some(t.clone())
                }));
            let (other_impl_generics, _, _) = other_generics.split_for_impl();

            let change_index = |(j, mut t): (_, TyParam), ident: String| {
                if i == j { t.ident = ident.into(); }
                t
            };

            let mut set_generics = item.generics.clone();
            add_ty_params(&mut set_generics, builder_ty_params.clone()
                .into_iter().enumerate()
                .map(|n| change_index(n, format!("{}::O", bmod))));
            let (_, set_ty_generics, _) = set_generics.split_for_impl();

            let mut after_set_generics = item.generics.clone();
            add_ty_params(&mut after_set_generics, builder_ty_params.clone()
                .into_iter().enumerate()
                .map(|n| change_index(n, format!("{}::I", bmod))));
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

fn unwrap_from_option(ty: &Ty) -> Ty {
    if let &Ty::Path(_, Path{ref segments, ..}) = ty {
        let &PathSegment{ref ident, ref parameters} = &segments[0];
        if ident == "Option" {
            if let &PathParameters::AngleBracketed(ref a) = parameters {
                return a.types[0].clone();
            }
        }
    }
    panic!("Tried to get inner type from non-Option.");
}

fn wrap_into_option(ty: Ty) -> Ty {
    let mut params = AngleBracketedParameterData::default();
    params.types.push(ty);
    Ty::Path(None, PathSegment {
            ident: Ident::new("Option"),
            parameters: PathParameters::AngleBracketed(params),
        }.into())
}

fn is_option(ty: &Ty) -> bool {
    if let &Ty::Path(_, ref p) = ty {
        if let Some(s) = p.segments.get(0) {
            return s.ident == "Option";
        }
    }
    false
}

fn collect_most_one<I, T>(mut iter: I, message: &'static str) -> Option<T>
    where I: Iterator<Item=T>
{
    let result = iter.next();
    assert!(iter.fuse().next().is_none(), message);
    result
}

fn get_builder_methods(attrs: &[Attribute]) -> (Ident, Ident) {
    let mut iter = attrs.iter()
        .filter_map(|a| {
            if let MetaItem::List(ref name, ref value) = a.value {
                if name == "builder_rename" {
                    return Some(value);
                }
            }
            None
        });
    collect_most_one(&mut iter, "Only one #[builder_rename] attribute supported per item.")
        .unwrap_or(&vec![])
        .iter()
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
        .fold((Ident::new("new"), Ident::new("build")), |(new, build), (first, v)| {
            if first {
                (v, build)
            } else {
                (new, v)
            }
        })
}

fn get_builder_name(attrs: &[Attribute]) -> Ident  {
    let mut iter = attrs.iter()
        .filter_map(|a| {
            if let MetaItem::NameValue(ref name, ref value) = a.value {
                if name == "builder_name" {
                    if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                        return Some(Ident::new(&value[..]));
                    }
                }
            }
            None
        });
    collect_most_one(&mut iter, "Only one #[builder_name] attribute supported per item.")
        .unwrap_or(Ident::new("Builder"))
}


fn get_setter_prefix(attrs: &[Attribute], default: Ident) -> Ident {
    let mut iter = attrs.iter()
        .filter_map(|a| {
            if let MetaItem::NameValue(ref name, ref value) = a.value {
                if name == "builder_prefix" {
                    if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                        return Some(Ident::new(&value[..]));
                    }
                }
            }
            None
        });
    collect_most_one(&mut iter, "Only one #[builder_prefix] attribute supported per item.")
        .unwrap_or(default)
}

fn plain_ty_param<I: Into<Ident>>(ident: I) -> TyParam {
    TyParam {
        ident: ident.into(),
        attrs: vec![],
        bounds: vec![],
        default: None,
    }
}

fn priv_field<I: Into<Ident>>(ident: I, ty: Ty) -> Field {
    Field {
        ident: Some(ident.into()),
        vis: Visibility::Inherited,
        attrs: vec![],
        ty: ty,
    }
}

fn add_ty_params<I: IntoIterator<Item=TyParam>>(generics: &mut Generics, ty_params: I) {
    let mut empty = vec![];
    swap(&mut empty, &mut generics.ty_params);
    generics.ty_params = ty_params.into_iter()
        .chain(empty)
        .collect();
}
