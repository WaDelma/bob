#![recursion_limit = "1024"]

//! # Bob the builder builder
//! Bob provides custom derive for generating builder for struct.
//!
//! It uses the type system to force user of the builder to provide all required fields.
//! # Examples
//! Basic usage:
//!
//! ````
//! #[macro_use]
//! extern crate bob;
//!
//! #[derive(Builder)]
//! #[builder_derive(Debug, Clone)]
//! struct MyStruct {
//!     // Greeting is optional
//!     greeting: Option<String>,
//!     // Magic numbers are required
//!     magics: Vec<i32>,
//! }
//!
//! fn main() {
//!     let my_struct = Builder::new()
//!         .magic(vec![42, 7, 3]) // This line is required. Removing it gives error.
//!         .greeting("Potato".to_owned()) // This line is optional.
//!                                        // Removing it will result in `None` in the final struct.
//!         .build();
//!     println!("Hello, {}!", my_struct.greeting.unwrap_or("World".to_owned()));
//!     println!("{} is the answer", my_struct.magic[0]);
//! }
//! ````
//! This example results in [this code](./fn.example_1_expanded.html) to be generated (after cleaning it up and adding comments).
//!
//! Renaming builder:
//!
//! ````
//! #[macro_use]
//! extern crate bob;
//!
//! #[derive(Builder)]
//! #[builder_names(builder = "MyBuilder", new = "create", build = "finish")]
//! #[builder_prefix = "set_"]
//! struct MyStruct<A, B> {
//!     firsts: Vec<A>,
//!     #[builder_prefix = "with_"]
//!     seconds: Vec<B>,
//! }
//!
//! fn main() {
//!     let my_struct = MyBuilder::create()
//!         .set_firsts(vec![1, 1, 2, 3, 5, 8])
//!         .with_seconds(vec![2, 3, 5, 7, 11])
//!         .finish();
//!     let a: i32 = my_struct.firsts.iter().sum();
//!     let b: i32 = my_struct.seconds.iter().sum();
//!     println!("{}", a + b);
//! }
//! ````
//! This example results in [this code](./fn.example_2_expanded.html) to be generated (after cleaning it up and adding comments).
//!
//! Validating fields:
//!
//! ````
//! #[macro_use]
//! extern crate bob;
//!
//! #[derive(Builder)]
//! #[builder_validate(validator = "MyStruct::validate", error = "BuildError")]
//! struct MyStruct {
//!     super_secret: String,
//! }
//!
//! enum BuildError {
//!     CatastrophicFailure
//! }
//!
//! impl MyStruct {
//!     fn validate(self) -> Result<Self, BuildError> {
//!         if self.super_secret == "00000000" {
//!             Ok(self)
//!         } else {
//!             Err(BuildError::CatastrophicFailure)
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let my_struct = Builder::new()
//!         .super_secret("password".to_owned())
//!         .build();
//!     if let Ok(_) = my_struct {
//!         println!("Access granted.");
//!     } else {
//!         println!("Permission denied!");
//!     }
//! }
//! ````
//! This example results in [this code](./fn.example_3_expanded.html) to be generated (after cleaning it up and adding comments).
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{Ident, Field, Ty, Lit, Generics, PolyTraitRef, TraitBoundModifier, TyParam, TyParamBound, Body, StrStyle, Attribute, Path, PathSegment, PathParameters, Visibility, MetaItem, NestedMetaItem, AngleBracketedParameterData, parse_path};

use std::mem::swap;
use std::fmt::Display;
use std::collections::HashSet;

use Globalization::*;

/// ````
/// # use std::fmt::{Debug, Error, Formatter};
/// # use std::mem::{uninitialized, forget, replace};
/// # use std::ptr::{write, read};
/// # use std::marker::PhantomData;
/// # use _mybuilder::*;
/// #[doc(hidden)]
/// #[allow(unused)]
/// mod _mybuilder {
///     // Indicates that value isn't set
///     pub struct O;
///     // Indicates that value is set
///     pub struct I;
///     // This function is is used for when user doesn't provide validation function.
///     pub fn id<T>(t: T) -> T {
///         t
///     }
/// }
/// /// Builder for `MyStruct`.
/// /// # Required fields
/// /// * `magics`
/// ///
/// /// # Optional fields
/// /// * `greeting`
/// ///
/// struct Builder<_0> {
///     _marker: PhantomData<(_0)>,
///     _f1: Option<Vec<i32>>,
///     _f0: Option<String>,
/// }
/// impl Builder<O> {
///     /// Constructor for builder.
///     ///
///     /// All fields are unset at the start.
///     fn new() -> Builder<O> {
///         Builder{
///             _marker: PhantomData,
///             _f1: None,
///             _f0: None,
///         }
///     }
/// }
/// impl <_0> Clone for Builder<_0> {
///     fn clone(&self) -> Self {
///         Builder {
///             _marker: PhantomData,
///             _f1: self._f0.clone(),
///             _f0: self._f0.clone(),
///         }
///     }
/// }
/// impl <_0> Debug for Builder<_0> {
///     fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
///         if let &BuilderInner::Inner { ref _f1, .. } = &self.inner {
///             fmt.debug_struct(stringify!(Builder))
///                 .field(stringify!(_f1), &self._f1)
///                 .field(stringify!(_f0), &self._f0)
///                 .finish()
///     }
/// }
/// impl Builder<I> {
///     /// Builds new `MyStruct`.
///     ///
///     /// This method is usable only if all required fields are set.
///     fn build(self) -> MyStruct {
///         id(MyStruct {
///              magic: self._f1.expect("Type parameters should ensure that there is always value."),
///              greeting: self._f0,
///         })
///     }
/// }
/// impl <_0> Builder<_0> {
///     /// Setter method for **optional** field `greeting`.
///     fn greeting(mut self, greeting: String) -> Builder<_0> {
///         self._f0 = Some(greeting);
///         self
///     }
/// }
/// impl Builder<O> {
///     /// Setter method for **required** field `magics`.
///     fn magics(self, magics: Vec<i32>) -> Builder<I> {
///         Builder {
///             _marker: PhantomData,
///             _f1: Some(magics),
///             _f0: self._f0,
///         }
///     }
/// }
/// ````
#[proc_macro_derive(Dummy1)]
pub fn example_1_expanded(_: TokenStream) -> TokenStream {unreachable!("Because there cannot be non-procmacro items in procmacro crate this hack is needed.")}
trait Dummy1 {}

/// ````
/// # use std::fmt::{Debug, Error, Formatter};
/// # use std::mem::{uninitialized, forget, replace};
/// # use std::ptr::{write, read};
/// # use std::marker::PhantomData;
/// # use _mybuilder::*;
/// #[doc(hidden)]
/// #[allow(unused)]
/// mod _mybuilder {
///     // Indicates that value isn't set
///     pub struct O;
///     // Indicates that value is set
///     pub struct I;
///     // This function is is used for when user doesn't provide validation function.
///     pub fn id<T>(t: T) -> T {
///         t
///     }
/// }
/// /// Builder for `MyStruct`.
/// /// # Required fields
/// /// * `firsts`
/// /// * `seconds`
/// ///
/// ///
/// struct MyBuilder<_0, _1, A, B> {
///     _marker: PhantomData<(_0, _1)>,
///     _f0: Vec<A>,
///     _f1: Vec<B>,
/// }
/// impl <A, B> MyBuilder<O, O, A, B> {
///     /// Constructor for builder.
///     ///
///     /// All fields are unset at the start.
///     fn create() -> MyBuilder<O, O, A, B> {
///         MyBuilder{
///             _marker: PhantomData,
///             _f0: None,
///             _f1: None,
///         }
///     }
/// }
/// impl <A, B> MyBuilder<I, I, A, B> {
///     /// Builds new `MyStruct`.
///     ///
///     /// This method is usable only if all required fields are set.
///     fn finish(self) -> MyStruct<A, B> {
///         id(MyStruct{
///             firsts: self._f0.expect("Type parameters should ensure that there is always value."),
///             seconds: self._f1.expect("Type parameters should ensure that there is always value."),
///         })
///     }
/// }
/// impl <_1, A, B> MyBuilder<O, _1, A, B> {
///     /// Setter method for **required** field `firsts`.
///     fn set_firsts(self, firsts: Vec<A>) -> MyBuilder<I, _1, A, B> {
///         Builder {
///             _marker: PhantomData,
///             _f0: Some(firsts),
///             _f1: self._f1,
///         }
///     }
/// }
/// impl <_0, A, B> MyBuilder<_0, O, A, B> {
///     /// Setter method for **required** field `seconds`.
///     fn with_seconds(self, seconds: Vec<B>) -> MyBuilder<_0, I, A, B> {
///         Builder {
///             _marker: PhantomData,
///             _f1: Some(seconds),
///             _f0: self._f0,
///         }
///     }
/// }
/// ````
#[proc_macro_derive(Dummy2)]
pub fn example_2_expanded(_: TokenStream) -> TokenStream {unreachable!("Because there cannot be non-procmacro items in procmacro crate this hack is needed.")}
trait Dummy2 {}

/// ````
/// # use std::fmt::{Debug, Error, Formatter};
/// # use std::mem::{uninitialized, forget, replace};
/// # use std::ptr::{write, read};
/// # use std::marker::PhantomData;
/// # use _mybuilder::*;
///
/// #[doc(hidden)]
/// #[allow(unused)]
/// mod _builder {
///     // Indicates that value isn't set
///     pub struct O;
///     // Indicates that value is set
///     pub struct I;
///     // This function is is used for when user doesn't provide validation function.
///     pub fn id<T>(t: T) -> T {
///         t
///     }
/// }
/// /// Builder for `MyStruct`.
/// /// # Required fields
/// /// * `super_secret`
/// ///
/// ///
/// struct Builder<_0> {
///     _marker: PhantomData<(_0)>,
///     _f0: Option<String>,
/// }
/// impl Builder<O> {
///     /// Constructor for builder.
///     ///
///     /// All fields are unset at the start.
///     fn new() -> Builder<O> {
///         Builder {
///             _marker: PhantomData,
///             _f0: None,
///         }
///     }
/// }
/// impl Builder<I> {
///     /// Builds new `MyStruct`.
///     ///
///     /// This method is usable only if all required fields are set.
///     fn build(self) -> Result<MyStruct, BuildError> {
///         MyStruct::validate(MyStruct{
///             super_secret: self._f0.expect("Type parameters should ensure that there is always value."),
///         })
///     }
/// }
/// impl Builder<O> {
///     ///Setter method for **required** field `super_secret`.
///     fn super_secret(self, super_secret: String) -> Builder<I> {
///         Builder {
///             _marker: PhantomData,
///             _f0: Some(super_secret),
///         }
///     }
/// }
/// ````
#[proc_macro_derive(Dummy3)]
pub fn example_3_expanded(_: TokenStream) -> TokenStream {unreachable!("Because there cannot be non-procmacro items in procmacro crate this hack is needed.")}
trait Dummy3 {}

/// Creates builder for struct annotated with 'Builder' attribute.
#[proc_macro_derive(Builder, attributes(builder_names, builder_prefix, builder_validate, builder_docs, builder_derive))]
pub fn create_builder(input: TokenStream) -> TokenStream {
    let item = syn::parse_derive_input(&input.to_string()).unwrap();
    if let Body::Struct(s) = item.body {
        let (builder, new, build) = get_builder_names(&item.attrs);
        let prefix = get_setter_prefix(&item.attrs, Ident::new(""));
        let derives = get_derives(&item.attrs);
        // This module holds types generated so they don't conflict with user added/generated by other invocations of this.
        let builder_mod = Ident::new(format!("_{}", builder.to_string().to_lowercase()));
        let (validator, validator_error) = get_validator(&item.attrs, format!("{}::id", builder_mod));

        let name = &item.ident;
        let vis = &item.vis;
        let (impl_generics, ty_generics, _) = item.generics.split_for_impl();

        // Fields need to be renamed so that they don't conficlict with _marker field.
        let (opt_fields, fields): (Vec<_>, Vec<_>)
            = s.fields()
                .iter()
                .enumerate()
                .map(|(i, f)| (Ident::new(format!("_f{}", i)), f))
                .partition(|&(_, ref f)| is_option(&f.ty));

        // Required fields are wrapped into Option so that they can have initial value.
        // TODO: When union becomes stable this can be implemented with uninitialized memory.
        let builder_fields = &fields.iter()
            .map(|&(ref i, ref f)| priv_field(i.clone(), wrap_into_option(f.ty.clone())))
            .collect::<Vec<_>>();
        let builder_field_names = &builder_fields.iter()
            .map(|f| f.ident.clone())
            .collect::<Vec<_>>();
        let builder_field_names2 = builder_field_names;
        // Optional values already have initial value.
        let builder_opt_fields = &opt_fields.iter()
            .map(|&(ref i, ref f)| priv_field(i.clone(), f.ty.clone()))
            .collect::<Vec<_>>();
        let builder_opt_field_names = &builder_opt_fields.iter()
            .map(|f| f.ident.clone())
            .collect::<Vec<_>>();
        let builder_opt_field_names2 = builder_opt_field_names;
        let result_fields = fields.iter().map(|&(_, f)| &f.ident);
        let result_opt_fields = opt_fields.iter().map(|&(_, f)| &f.ident);

        let builder_plain_ty_params = &(0..builder_fields.len())
            .map(|i| plain_ty_param(format!("_{}", i)))
            .collect::<Vec<_>>();
        // Type parameters for builders required fields
        let builder_ty_params = &(0..builder_fields.len())
            .map(|i| plain_ty_param(format!("_{}", i)))
            .collect::<Vec<_>>();

        // All type parameters that the builder has.
        let mut ext_generics = item.generics.clone();
        add_ty_params(&mut ext_generics, builder_ty_params.clone());
        let (ext_impl_generics, ext_ty_generics, ext_where_clause) = ext_generics.split_for_impl();

        // All type parameters that the builder has with additional Clone bound.
        let mut ext_clone_generics = item.generics.clone();
        ext_clone_generics.ty_params = ext_clone_generics.ty_params
            .into_iter()
            .map(|mut ty| {
                ty.bounds.push(ty_param_bound(vec!["Clone".into()], Local));
                ty
            })
            .collect::<Vec<_>>();
        add_ty_params(&mut ext_clone_generics, builder_ty_params.clone());
        let (ext_clone_impl_generics, _, _) = ext_clone_generics.split_for_impl();

        // All type parameters that the builder has with additional Debug bound.
        let mut ext_debug_generics = item.generics.clone();
        ext_debug_generics.ty_params = ext_debug_generics.ty_params
            .into_iter()
            .map(|mut ty| {
                ty.bounds.push(ty_param_bound(vec!["std".into(), "fmt".into(), "Debug".into()], Global));
                ty
            })
            .collect::<Vec<_>>();
        add_ty_params(&mut ext_debug_generics, builder_ty_params.clone());
        let (ext_debug_impl_generics, _, _) = ext_debug_generics.split_for_impl();

        // Type parameters for constructor.
        // At the start builder doesn't have any values set.
        let mut start_generics = item.generics.clone();
        add_ty_params(&mut start_generics,
            (0..builder_fields.len())
                .map(|_| plain_ty_param(format!("{}::O", builder_mod))));
        let (_, start_ty_generics, start_where_clause) = start_generics.split_for_impl();

        // Type parameters for build method.
        // When building we require that every required value is set.
        let mut end_generics = item.generics.clone();
        add_ty_params(&mut end_generics,
            (0..builder_fields.len())
                .map(|_| plain_ty_param(format!("{}::I", builder_mod))));
        let (_, end_ty_generics, _) = end_generics.split_for_impl();

        let required = if fields.is_empty() {
            "".into()
        } else {
            fields.iter()
                .cloned()
                .map(|(i, f)| f.ident.clone().unwrap_or((&i.as_ref()[1..]).into()))
                .map(|i| format!("* `{}`\n", i))
                .fold("# Required fields\n".to_owned(), |a, b| a + &b)
        };
        let optional = if opt_fields.is_empty() {
            "".into()
        } else {
            opt_fields.iter()
                .cloned()
                .map(|(i, f)| f.ident.clone().unwrap_or((&i.as_ref()[1..]).into()))
                .map(|i| format!("* `{}`\n", i))
                .fold("# Optional fields\n".to_owned(), |a, b| a + &b)
        };

        let builder_doc = format!("Builder for `{}`.\n{}\n{}", name, required, optional);
        let constructor_doc = format!("Constructor for builder.\n\nAll fields are unset at the start.");
        let build_doc = format!("Builds new `{}`.\n\nThis method is usable only if all required fields are set.", name);
        let mut tks = quote!(
            #[doc(hidden)]
            #[allow(unused)]
            #vis mod #builder_mod {
                // Indicates that value isn't set
                pub struct O;
                // Indicates that value is set
                pub struct I;
                // This function is is used for when user doesn't provide validation function.
                pub fn id<T>(t: T) -> T {t}
            }

            #[doc = #builder_doc]
            #vis struct #builder #ext_impl_generics #ext_where_clause {
                _marker: ::std::marker::PhantomData<(#(#builder_plain_ty_params),*)>,
                #(#builder_fields,)*
                #(#builder_opt_fields),*
            }

            impl #impl_generics #builder #start_ty_generics #start_where_clause {
                #[doc = #constructor_doc]
                #vis fn #new() -> #builder #start_ty_generics {
                    #builder {
                        _marker: ::std::marker::PhantomData,
                        #(#builder_field_names: None,)*
                        #(#builder_opt_field_names: None),*
                    }
                }
            }
        );

        if derives.contains("Clone") {
            let parsed: String = quote!(
                impl #ext_clone_impl_generics Clone for #builder #ext_ty_generics #ext_where_clause {
                    fn clone(&self) -> Self {
                        #builder {
                            _marker: ::std::marker::PhantomData,
                            #(#builder_field_names: self.#builder_field_names2.clone(),)*
                            #(#builder_opt_field_names: self.#builder_opt_field_names2.clone()),*
                        }
                    }
                }
            ).parse().unwrap();
            tks.append(&parsed);
        }

        if derives.contains("Debug") {
            let parsed: String = quote!(
                impl #ext_debug_impl_generics ::std::fmt::Debug for #builder #ext_ty_generics #ext_where_clause {
                    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
                        fmt.debug_struct(stringify!(#builder))
                            #(.field(stringify!(#builder_field_names), &self.#builder_field_names2))*
                            #(.field(stringify!(#builder_opt_field_names), &self.#builder_opt_field_names2))*
                            .finish()
                    }
                }
            ).parse().unwrap();
            tks.append(&parsed);
        }

        let parsed: String = if let Some(error) = validator_error {
            quote!(
                impl #impl_generics #builder #end_ty_generics
                    #ext_where_clause
                {
                    #[doc = #build_doc]
                    #vis fn #build(self) -> Result<#name #ty_generics, #error> {
                        #validator(#name {
                            #(#result_fields: self.#builder_field_names.expect("Type parameters should ensure that there is always value."),)*
                            #(#result_opt_fields: self.#builder_opt_field_names),*
                        })
                    }
                }
            ).parse().unwrap()
        } else {
            quote!(
                impl #impl_generics #builder #end_ty_generics
                    #ext_where_clause
                {
                    #[doc = #build_doc]
                    #vis fn #build(self) -> #name #ty_generics {
                        #validator(#name {
                            #(#result_fields: self.#builder_field_names.expect("Type parameters should ensure that there is always value."),)*
                            #(#result_opt_fields: self.#builder_opt_field_names),*
                        })
                    }
                }
            ).parse().unwrap()
        };
        tks.append(&parsed);

        for (i, &(ref fname, ref field)) in opt_fields.iter().enumerate() {
            // This being optional field doesn't mean that the setter takes optional.
            let ty = unwrap_from_option(&field.ty).expect("Tried to get inner type from non-Option.");

            // Per field prefixes are supported
            let prefix = get_setter_prefix(&field.attrs, prefix.clone());
            let raw_name = field.ident.clone().unwrap_or_else(|| i.to_string().into());
            let name = Ident::new(&format!("{}{}", prefix, raw_name)[..]);

            let setter_doc = format!("Setter method for **optional** field `{}`.", raw_name);
            let parsed: String = quote!(
                impl #ext_impl_generics #builder #ext_ty_generics #ext_where_clause {
                    #[doc = #setter_doc]
                    #vis fn #name(mut self, #raw_name: #ty) -> #builder #ext_ty_generics {
                        self.#fname = Some(#raw_name);
                        self
                    }
                }
            ).parse().unwrap();
            tks.append(&parsed);
        }

        for (i, &(ref fname, ref field)) in fields.iter().enumerate() {
            let ty = &field.ty;

            // All but the field that the setter is setting needs to be just taken from self.
            let mut builder_field_names = builder_field_names.clone();
            builder_field_names.remove(i);
            let builder_field_names = &builder_field_names;
            let builder_field_names2 = builder_field_names;

            // Per field prefixes are supported
            let prefix = get_setter_prefix(&field.attrs, prefix.clone());
            let raw_name = field.ident.clone().unwrap_or_else(|| i.to_string().into());
            let name = Ident::new(&format!("{}{}", prefix, raw_name)[..]);

            // Because one concrete type parameter is used, all but one unbound one is needed.
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

            // Fields can be set only once, so we require that field wasn't set before.
            let mut set_generics = item.generics.clone();
            add_ty_params(&mut set_generics, builder_ty_params.clone()
                .into_iter().enumerate()
                .map(|n| change_index(n, format!("{}::O", builder_mod))));
            let (_, set_ty_generics, _) = set_generics.split_for_impl();

            // After setting field, type parameter is changed to indicate that.
            let mut after_set_generics = item.generics.clone();
            add_ty_params(&mut after_set_generics, builder_ty_params.clone()
                .into_iter().enumerate()
                .map(|n| change_index(n, format!("{}::I", builder_mod))));
            let (_, after_set_ty_generics, _) = after_set_generics.split_for_impl();

            let setter_doc = format!("Setter method for **required** field `{}`.", raw_name);
            let parsed: String = quote!(
                impl #other_impl_generics #builder #set_ty_generics #ext_where_clause {
                    #[doc = #setter_doc]
                    #vis fn #name(self, #raw_name: #ty) -> #builder #after_set_ty_generics {
                        #builder {
                            _marker: ::std::marker::PhantomData,
                            #fname: Some(#raw_name),
                            #(#builder_field_names: self.#builder_field_names2,)*
                            #(#builder_opt_field_names: self.#builder_opt_field_names2),*
                        }
                    }
                }
            ).parse().unwrap();
            tks.append(&parsed);
        }
        debug_display(tks.parse().unwrap())
    } else {
        panic!("Only structs supported.");
    }
}

#[inline(always)]
fn debug_display<T: Display>(t: T) -> T {
    //println!("{}", t);
    t
}

/// Wraps type T into Option<T>
fn wrap_into_option(ty: Ty) -> Ty {
    let mut params = AngleBracketedParameterData::default();
    params.types.push(ty);
    Ty::Path(None, PathSegment {
            ident: Ident::new("Option"),
            parameters: PathParameters::AngleBracketed(params),
        }.into())
}


/// Returns inner type T of Option<T> or None if type wasn't Option.
fn unwrap_from_option(ty: &Ty) -> Option<&Ty> {
    if let &Ty::Path(_, Path{ref segments, ..}) = ty {
        let &PathSegment{ref ident, ref parameters} = &segments[0];
        if ident == "Option" {
            if let &PathParameters::AngleBracketed(ref a) = parameters {
                return a.types.get(0)
            }
        }
    }
    None
}

/// Checks if give type is Option
fn is_option(ty: &Ty) -> bool {
    if let &Ty::Path(_, ref p) = ty {
        if let Some(s) = p.segments.get(0) {
            return s.ident == "Option";
        }
    }
    false
}

/// Collects iterators next element and panics with message if there is still elements left after that.
fn collect_most_one<I, T>(mut iter: I, message: &'static str) -> Option<T>
    where I: Iterator<Item=T>
{
    let result = iter.next();
    assert!(iter.fuse().next().is_none(), message);
    result
}

enum Named {
    Builder,
    New,
    Build,
}

impl Named {
    fn from_str(s: &str) -> Option<Named> {
        use Named::*;
        match s {
            "builder" => Some(Builder),
            "new" => Some(New),
            "build" => Some(Build),
            _ => None,
        }
    }
}

/// Gets builders, builders constructors and build methods names based on attribute and falls back to default ones if no attribute present.
fn get_builder_names(attrs: &[Attribute]) -> (Ident, Ident, Ident) {
    let mut iter = attrs.iter()
        .filter_map(|a| {
            if let MetaItem::List(ref name, ref value) = a.value {
                if name == "builder_names" {
                    return Some(value);
                }
            }
            None
        });
    collect_most_one(&mut iter, "Only one #[builder_name] attribute supported for struct.")
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            if let &NestedMetaItem::MetaItem(MetaItem::NameValue(ref name, ref value)) = v {
                if let Some(which) = Named::from_str(name.as_ref()) {
                    if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                        return Some((which, Ident::new(&value[..])));
                    }
                }
            }
            None
        })
        .fold((Ident::new("Builder"), Ident::new("new"), Ident::new("build")), |(builder, new, build), (which, v)| {
            use Named::*;
            match which {
                Builder => (v, new, build),
                New => (builder, v, build),
                Build => (builder, new, v),
            }
        })
}

fn get_derives(attrs: &[Attribute]) -> HashSet<String> {
    attrs.iter()
        .flat_map(|a| {
            if let MetaItem::List(ref name, ref value) = a.value {
                if name == "builder_derive" {
                    return value.iter()
                        .filter_map(|v| {
                            if let &NestedMetaItem::MetaItem(ref meta) = v {
                                if let &MetaItem::Word(ref ident) = meta {
                                    return Some(ident.as_ref().to_owned());
                                }
                            }
                            None
                        })
                        .collect();
                }
            }
            vec![]
        })
        .collect()
}

/// Gets setter prefix based on attribute and falls back to default given as parameter if no attribute present.
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

/// Gets validator function and error type based on attribute and falls back to default if there isn't one.
fn get_validator<P: Into<Path>>(attrs: &[Attribute], default: P) -> (Path, Option<Path>) {
    let mut iter = attrs.iter()
        .filter_map(|a| {
            if let MetaItem::List(ref name, ref value) = a.value {
                if name == "builder_validate" {
                    return Some(value);
                }
            }
            None
        });
    let result = collect_most_one(&mut iter, "Only one #[builder_validate] attribute supported for struct.");
    if let Some(r) = result {
        let (v, e) = r.iter()
        .filter_map(|v| {
            if let &NestedMetaItem::MetaItem(MetaItem::NameValue(ref name, ref value)) = v {
                if name == "validator" || name == "error" {
                    if let &Lit::Str(ref value, StrStyle::Cooked) = value {
                        return Some((name == "validator", parse_path(&value[..]).expect("Malformed path given to `builder_validate` attribute")));
                    }
                }
            }
            None
        })
        .fold((None, None), |(validator, error), (first, v)| {
            if first {
                (Some(v), error)
            } else {
                (validator, Some(v))
            }
        });
        (v.expect("Validator function has to be provided for `builder_validate` attribute."), e)
    } else {
        (default.into(), None)
    }
}


/// Constructs type parameter without bounds from identifier.
fn plain_ty_param<I: Into<Ident>>(ident: I) -> TyParam {
    TyParam {
        ident: ident.into(),
        attrs: vec![],
        bounds: vec![],
        default: None,
    }
}

/// Constructs private field from identifier and type.
fn priv_field<I: Into<Ident>>(ident: I, ty: Ty) -> Field {
    Field {
        ident: Some(ident.into()),
        vis: Visibility::Inherited,
        attrs: vec![],
        ty: ty,
    }
}

/// Adds type parameters to the start of generics.
fn add_ty_params<I: IntoIterator<Item=TyParam>>(generics: &mut Generics, ty_params: I) {
    let mut empty = vec![];
    swap(&mut empty, &mut generics.ty_params);
    generics.ty_params = ty_params.into_iter()
        .chain(empty)
        .collect();
}

#[derive(PartialEq, Eq)]
enum Globalization {
    Global,
    Local,
}

/// Creates type parameter bound based on path segments
fn ty_param_bound(segments: Vec<PathSegment>, global: Globalization) -> TyParamBound {
    TyParamBound::Trait(
        PolyTraitRef{
            bound_lifetimes: vec![],
            trait_ref: Path{
                global: global == Global,
                segments: segments
            }
        },
        TraitBoundModifier::None
    )
}
