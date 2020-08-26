use syn::{
    token, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput,
    Meta,
    Path, PathSegment, PathArguments,
    Ident,
    Lit, LitStr,
};
use quote::quote;
use fehler::{throws, throw};
use smallvec::SmallVec;
use proc_macro2::{TokenStream, Span};
use crate::matching::*;

#[throws(syn::Error)]
pub fn derive_config_table_expand(item: DeriveInput) -> TokenStream {
    let struct_name = item.ident;
    let struct_data = typedata_struct(item.data)?;
    let fields = fields_named(struct_data.fields)?;
    let (default_receiver, generated_entry_module_path) = {
        let mut receiver = None;
        let mut generated_entry_module_path = None;
        for attribute in item.attrs {
            let outcomes = handle_struct_attribute(attribute.parse_meta()?)?;
            for outcome in outcomes {
                match outcome {
                    StructAttributeOutcome::SetDefaultReceiver(x) => {
                        if receiver.is_none() {
                            receiver = Some(x);
                        } else {
                            throw!(syn::Error::new(
                                Span::call_site(),
                                "cannot set multiple default receivers at once",
                            ));
                        }
                    },
                    StructAttributeOutcome::SetGenEntryModule(x) => {
                        if generated_entry_module_path.is_none() {
                            generated_entry_module_path = Some(x);
                        } else {
                            throw!(syn::Error::new(
                                Span::call_site(),
                                "cannot set multiple modules for generated entry markers at once",
                            ));
                        }
                    },
                    StructAttributeOutcome::Ignore => {},
                }
            }
        }
        (
            receiver.unwrap_or_else(default_receiver),
            Path {
                leading_colon: None,
                segments: {
                    let mut segments = Punctuated::new();
                    segments.push(
                        generated_entry_module_path.map_or_else (
                            || Ident::new("entries", Span::call_site()).into(),
                            PathSegment::from,
                        )
                    );
                    segments
                }
            },
        )
    };
    let mut requested_get_impls = Vec::new();
    let mut requested_generated_entries = Vec::new();
    for field in fields.named {
        let field_ident = field.ident.unwrap();
        let field_type = field.ty;
        for attribute in field.attrs {
            let outcomes = handle_field_attribute(attribute.parse_meta()?)?;
            for outcome in outcomes {
                match outcome {
                    FieldAttributeOutcome::RequestGetImpl {
                        entry_name,
                        receiver_name,
                    } => {
                        requested_get_impls.push((
                            field_ident.clone(),
                            entry_name,
                            receiver_name.unwrap_or_else(|| default_receiver.clone()),
                        ));
                    },
                    FieldAttributeOutcome::CreateEntryAndGetImpl {
                        entry_name,
                        receiver_name,
                    } => {
                        requested_get_impls.push((
                            field_ident.clone(),
                            path_to_generated_entry (
                                generated_entry_module_path.clone(),
                                entry_name.clone(),
                                &field_ident,
                            ),
                            receiver_name.unwrap_or_else(|| default_receiver.clone()),
                        ));
                        requested_generated_entries.push((
                            field_ident.clone(),
                            entry_name.unwrap_or_else(|| snake_to_camel(field_ident.clone())),
                            field_type.clone(),
                        ));
                    },
                    FieldAttributeOutcome::Ignore => {},
                }
            }
        }
    }
    let mut get_impl_token_streams = Vec::new();
    for (field_ident, entry_path, receiver) in requested_get_impls {
        let receiver_type = receiver.annotation;
        let receiver_fn = receiver.receiver_fn;
        let token_stream = quote! {
            impl ::snec::Get<#entry_path> for #struct_name {
                type Receiver = #receiver_type;
                #[inline(always)]
                fn get_ref(&self) -> &<#entry_path as ::snec::Entry>::Data {
                    &self.#field_ident
                }
                #[inline]
                fn get_handle(&mut self) -> ::snec::Handle<'_, #entry_path, #receiver_type> {
                    ::snec::Handle::new(&mut self.#field_ident, #receiver_fn())
                }
            }
        };
        get_impl_token_streams.push(token_stream);
    }
    let mut generated_entries = Vec::new();
    for (field_ident, entry_name, data_type) in requested_generated_entries {
        let documentation = format!(
            "The entry identifier type for the `{}` field in the `{}` config table.",
            &field_ident,
            &struct_name,
        );
        let documentation = Lit::Str(
            LitStr::new(&documentation, Span::call_site()),
        );
        let field_name_literal = Lit::Str(
            LitStr::new(&field_ident.to_string(), Span::call_site()),
        );
        let token_stream = quote! {
            #[doc = #documentation]
            pub enum #entry_name {}
            impl ::snec::Entry for #entry_name {
                type Data = #data_type;
                const NAME: &'static str = #field_name_literal;
            }
        };
        generated_entries.push(token_stream);
    }
    let result = quote! {
        mod #generated_entry_module_path {
            use super::*; // To make `type Data = ...` work in generated entry types
            #(#generated_entries)*
        }
        #(#get_impl_token_streams)*
    };
    result
}

fn default_receiver() -> AnnotatedReceiverFn {
    // FIXME this is shit
    let receiver_fn_segments = ["snec", "EmptyReceiver", "new"].iter().copied().map(
        |x| PathSegment {ident: Ident::new(x, Span::call_site()), arguments: PathArguments::None}
    ).collect();
    let annotation_segments = ["snec", "EmptyReceiver"].iter().copied().map(
        |x| PathSegment {ident: Ident::new(x, Span::call_site()), arguments: PathArguments::None}
    ).collect();
    AnnotatedReceiverFn{
        receiver_fn: Path {
            leading_colon: Some(token::Colon2 {spans: [Span::call_site(); 2]}),
            segments: receiver_fn_segments,
        },
        _arrow: token::RArrow {spans: [Span::call_site(); 2]},
        annotation: Path {
            leading_colon: Some(token::Colon2 {spans: [Span::call_site(); 2]}),
            segments: annotation_segments,
        },
    }
}

enum StructAttributeOutcome {
    SetDefaultReceiver(AnnotatedReceiverFn),
    SetGenEntryModule(Ident),
    Ignore,
}
#[throws(syn::Error)]
fn handle_struct_attribute(attribute: Meta) -> SmallVec<[StructAttributeOutcome; 1]> {
    let meta_list = match attribute {
        Meta::List(x) => {
            if x.path.is_ident("snec") {x}
            else {
                // Not our attribute, ignoring
                return SmallVec::from([StructAttributeOutcome::Ignore])
            }
        },
        Meta::Path(x) => {
            if x.is_ident("snec") {
                throw!(syn::Error::new_spanned(x, "empty `#[snec]` attributes are not allowed"))
            } else {
                return SmallVec::from([StructAttributeOutcome::Ignore])
            }
        },
        Meta::NameValue(x) => {
            if x.path.is_ident("snec") {
                throw!(
                    syn::Error::new_spanned(x, "`#[snec = \"...\"]` attributes are not allowed")
                )
            } else {
                return SmallVec::from([StructAttributeOutcome::Ignore])
            }
        },
    };
    let mut result = SmallVec::new();
    for action in meta_list.nested {
        let meta = nested_meta_normal(action)?;
        result.push(handle_snec_struct_attribute(meta)?);
    }
    result
}
#[throws(syn::Error)]
fn handle_snec_struct_attribute(attribute: Meta) -> StructAttributeOutcome {
    let name_value = match attribute {
        Meta::NameValue(x) => x,
        Meta::List(x) => {
            throw!(
                syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
            )
        },
        Meta::Path(x) => {
            throw!(
                syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
            )
        },
    };
    if name_value.path.is_ident("receiver") {
        let receiver = lit_str(name_value.lit)?.parse()?;
        StructAttributeOutcome::SetDefaultReceiver(receiver)
    } else if name_value.path.is_ident("entry_module") {
        let module_name = lit_str(name_value.lit)?.parse()?;
        StructAttributeOutcome::SetGenEntryModule(module_name)
    } else {
        throw!(syn::Error::new_spanned(name_value, "unknown `#[snec(...)]` sub-attribute"))
    }
}

enum FieldAttributeOutcome {
    RequestGetImpl {
        entry_name: Path,
        receiver_name: Option<AnnotatedReceiverFn>,
    },
    CreateEntryAndGetImpl {
        entry_name: Option<Ident>,
        receiver_name : Option<AnnotatedReceiverFn>,
    },
    // TODO: "Field forwarding", which would be a `#[snec(...)]` attribute on a field which is
    // itself a struct and would use a field of that struct as a source for an entry.
    /*RequestForwardedGetImpls {
        /// All fields for which forwarding is requested. The first item in the tuple is the name of the field to be forwarded, the second one is the `Entry` implementor and the third one is the optional receiver constructor function.
        forward_fields: Vec<(Ident, Ident, Option<Ident>)>,
    },*/
    Ignore,
}
#[throws(syn::Error)]
fn handle_field_attribute(attribute: Meta) -> SmallVec<[FieldAttributeOutcome; 1]> {
    let meta_list = match attribute {
        Meta::List(x) => {
            if x.path.is_ident("snec") {x}
            else {
                // Not our attribute, ignoring
                return SmallVec::from([FieldAttributeOutcome::Ignore])
            }
        },
        Meta::Path(x) => {
            if x.is_ident("snec") {
                return SmallVec::from([FieldAttributeOutcome::CreateEntryAndGetImpl {
                    entry_name: None, receiver_name: None,
                }]);
            } else {
                return SmallVec::from([FieldAttributeOutcome::Ignore])
            }
        },
        Meta::NameValue(x) => {
            if x.path.is_ident("snec") {
                throw!(
                    syn::Error::new_spanned(x, "`#[snec = \"...\"]` attributes are not allowed")
                )
            } else {
                return SmallVec::from([FieldAttributeOutcome::Ignore])
            }
        },
    };
    let mut result = SmallVec::new();
    for action in meta_list.nested {
        let meta = nested_meta_normal(action)?;
        result.push(handle_snec_field_attribute(meta)?);
    }
    result
}
#[throws(syn::Error)]
fn handle_snec_field_attribute(attribute: Meta) -> FieldAttributeOutcome {
    let meta_list = match attribute {
        Meta::List(x) => x,
        Meta::NameValue(x) => {
            throw!(
                syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
            )
        },
        Meta::Path(x) => {
            throw!(
                syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
            )
        },
    };
    if meta_list.path.is_ident("use_entry") {
        let num_args = meta_list.nested.len();
        if !(num_args == 1 || num_args == 2) {
            throw!(
                syn::Error::new_spanned(meta_list, "wrong number of arguments (expected 1 or 2)")
            )
        }
        let entry_name = meta_path( nested_meta_normal(meta_list.nested[0].clone())? )?;
        let receiver_name = if num_args == 2 {
            Some(lit_str( nested_meta_literal(meta_list.nested[1].clone())? )?.parse()?)
        } else {None};
        FieldAttributeOutcome::RequestGetImpl {entry_name, receiver_name}
    } else if meta_list.path.is_ident("entry") {
        let num_args = meta_list.nested.len();
        if num_args > 2 {
            throw!(
                syn::Error::new_spanned(meta_list, "wrong number of arguments (expected 0, 1 or 2)")
            )
        }
        let (mut entry_name, mut receiver_name) = (None, None);
        for argument in meta_list.nested {
            // FIXME remove this clone
            if let Ok(x) = nested_meta_normal(argument.clone()) {
                if entry_name.is_none() {
                    entry_name = Some({
                        let path = meta_path(x)?;
                        if path.segments.len() != 1 || path.leading_colon.is_some() {
                            throw!(syn::Error::new_spanned(
                                path,
                                "unrecognized `#[snec(...)]` attribute syntax"
                            ))
                        }
                        path.segments[0].clone().ident
                    });
                } else {
                    throw!(syn::Error::new(
                        Span::call_site(),
                        "cannot specify entry marker type name twice",
                    ))
                }
            }
            if let Ok(x) = nested_meta_literal(argument) {
                if receiver_name.is_none() {
                    receiver_name = Some({
                        let literal = lit_str(x)?;
                        literal.parse()?
                    })
                } else {
                    throw!(syn::Error::new(
                        Span::call_site(),
                        "cannot specify receiver source twice",
                    ))
                }
            }
        }
        FieldAttributeOutcome::CreateEntryAndGetImpl {entry_name, receiver_name}
    } else {
        throw!(syn::Error::new_spanned(meta_list, "unknown `#[snec(...)]` sub-attribute"))
    }
}

#[derive(Clone)]
struct AnnotatedReceiverFn {
    receiver_fn: Path,
    _arrow: Token![->],
    annotation: Path,
}
impl Parse for AnnotatedReceiverFn {
    #[throws(syn::Error)]
    fn parse(input: ParseStream) -> Self {
        Self {
            receiver_fn: input.parse()?,
            _arrow: input.parse()?,
            annotation: input.parse()?,
        }
    }
}

#[inline]
pub fn snake_to_camel(ident: Ident) -> Ident {
    let span = ident.span();
    let ident = ident.to_string();
    // It's better to do an excessively big allocation than to reallocate.
    let mut result = String::with_capacity(ident.len());
    ident.chars().fold(true, |previous_was_underscore, x| {
        if x == '_' {
            true
        } else {
            let x = if previous_was_underscore {
                x.to_uppercase().next().unwrap()
            } else {x};
            result.push(x);
            false
        }
    });
    Ident::new(&result, span)
}
pub fn path_to_generated_entry (
    generated_module: Path,
    entry_name: Option<Ident>,
    field_ident: &Ident,
) -> Path {
    Path {
        leading_colon: generated_module.leading_colon,
        segments: {
            let mut segments = Punctuated::new();
            segments.extend(generated_module.segments);
            segments.push(entry_name.unwrap_or_else(
                || snake_to_camel(field_ident.clone())
            ).into());
            segments
        },
    }
}