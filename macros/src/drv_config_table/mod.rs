mod ast {
    mod snec_attribute;
    pub use snec_attribute::*;
    mod struct_input;
    pub use struct_input::*;
}
use ast::*;

mod misc;
use misc::*;

use std::convert::TryFrom;
use syn::{
    Attribute,
    Expr,
    ExprPath,
    Path,
    PathSegment,
    PathArguments,
    Type,
    TypePath,
    Lit,
    LitStr,
    Visibility,
    punctuated::Punctuated,
    token,
};
use proc_macro2::{TokenStream, Span, Ident};
use quote::quote;

pub fn derive_config_table_expand(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let struct_input = syn::parse2::<ConfigTableStruct>(input)?;
    let (
        default_receiver_expr,
        default_receiver_type,
        entry_module,
        entry_module_visibility,
        entry_module_attributes,
    ) = {
        let mut receiver_expr = None;
        let mut receiver_type = None;
        let mut entry_module = None;
        let mut entry_module_visibility = None;
        let mut entry_module_attributes = Vec::new();
        for attr in filter_to_snec_attributes(struct_input.attrs) {
            let body = if let Some(body) = attr.body {
                body
            } else {
                return Err(
                    syn::Error::new(
                        attr.path_span,
                        "bare `#[snec]` attribute cannot be applied to whole struct",
                    )
                )
            };
            for command in body.commands {
                match command {
                    AttributeCommand::EntryModule { value, .. } => {
                        entry_module = Some(value);
                    },
                    AttributeCommand::EntryModuleVisibility { value, ..} => {
                        entry_module_visibility = Some(value);
                    },
                    AttributeCommand::EntryModuleAttributes { value, .. } => {
                        entry_module_attributes.extend(value);
                    },
                    AttributeCommand::Receiver { expression, ty, .. } => {
                        receiver_expr = Some(expression);
                        receiver_type = Some(ty);
                    },
                    AttributeCommand::Entry { name, .. } => {
                        return Err(
                            syn::Error::new(
                                name.0,
                                "\
`#[snec(entry(...))]` attribute cannot be applied to whole struct",
                            )
                        )
                    },
                    AttributeCommand::UseEntry { name, .. } => {
                        return Err(
                            syn::Error::new(
                                name.0,
                                "\
`#[snec(use_entry(...))]` attribute cannot be applied to whole struct",
                            )
                        )
                    },
                }
            }
        }
        (
            receiver_expr.unwrap_or_else(default_receiver_expr),
            receiver_type.unwrap_or_else(default_receiver_type),
            entry_module.unwrap_or_else(default_entry_module),
            entry_module_visibility.unwrap_or(Visibility::Inherited),
            entry_module_attributes,
        )
    };
    let mut requested_get_impls = Vec::with_capacity(struct_input.fields.len());
    let mut requested_generated_entries = Vec::with_capacity(struct_input.fields.len());
    for field in struct_input.fields {
        let field_ident = field.ident.unwrap();
        for attr in filter_to_snec_attributes(field.attrs) {
            let commands = {
                if let Some(body) = attr.body {
                    body.commands.into_iter().into()
                } else {
                    AttributeCommandIter::from(AttributeCommand::default())
                }
            };
            let mut generate_get_impl = false;
            let mut custom_marker_path = None;
            let mut generate_entry = false;
            let mut custom_marker_name = None;
            let mut custom_receiver_expr = None;
            let mut custom_receiver_type = None;
            for command in commands {
                match command {
                    AttributeCommand::Entry { value, .. } => {
                        if let Some(marker_name) = value {
                            custom_marker_path = Some(
                                concat_to_path(entry_module.clone(), marker_name.clone())
                            );
                            custom_marker_name = Some(marker_name);
                        }
                        generate_get_impl = true;
                        generate_entry = true;
                    },
                    AttributeCommand::UseEntry { value, .. } => {
                        generate_get_impl = true;
                        custom_marker_path = Some(value);
                    },
                    AttributeCommand::Receiver { expression, ty, .. } => {
                        custom_receiver_expr = Some(expression);
                        custom_receiver_type = Some(ty);
                    },
                    AttributeCommand::EntryModule { name, .. } => {
                        return Err(
                            syn::Error::new(
                                name.0,
                                "\
the `#[snec(entry_module(...))]` attribute can only be applied to the whole struct",
                            )
                        )
                    },
                    AttributeCommand::EntryModuleVisibility { name, .. } => {
                        return Err(
                            syn::Error::new(
                                name.0,
                                "\
the `#[snec(entry_module_visibility(...))]` attribute can only be applied to the whole struct",
                            )
                        )
                    },
                    AttributeCommand::EntryModuleAttributes { name, .. } => {
                        return Err(
                            syn::Error::new(
                                name.0,
                                "\
the `#[snec(entry_module_attributes(...))]` attribute can only be applied to the whole struct",
                            )
                        )
                    },
                }
            }
            if generate_entry {
                requested_generated_entries.push(
                    RequestedGeneratedEntry {
                        field_name: field_ident.clone(),
                        field_type: field.ty.clone(),
                        marker_name: custom_marker_name.unwrap_or_else(
                            || snake_to_camel(field_ident.clone())
                        ),
                    }
                )
            }
            if generate_get_impl {
                requested_get_impls.push(
                    RequestedGetImpl {
                        field_name: field_ident.clone(),
                        receiver_expr: custom_receiver_expr.unwrap_or_else(
                            || default_receiver_expr.clone()
                        ),
                        receiver_type: custom_receiver_type.unwrap_or_else(
                            || default_receiver_type.clone()
                        ),
                        marker_path: custom_marker_path.unwrap_or_else(
                            || concat_to_path(
                                entry_module.clone(),
                                snake_to_camel(field_ident.clone()),
                            )
                        ),
                        
                    }
                )
            }
        }
    }
    let mut impls = Vec::with_capacity(
        requested_get_impls.len() + requested_generated_entries.len()
    );
    let mut generated_entries = Vec::with_capacity(requested_generated_entries.len());
    for get_impl_data in requested_get_impls {
        let entry_path = get_impl_data.marker_path;
        let field_ident = get_impl_data.field_name;
        let receiver_expr = get_impl_data.receiver_expr;
        let receiver_type = get_impl_data.receiver_type;
        let struct_name = &struct_input.ident;
        let token_stream = quote! {
            impl ::snec::Get<#entry_path> for #struct_name {
                type Receiver = #receiver_type;
                #[inline(always)]
                fn get_ref(&self) -> &<#entry_path as ::snec::Entry>::Data {
                    &self.#field_ident
                }
                #[inline]
                fn get_handle(&mut self) -> ::snec::Handle<'_, #entry_path, #receiver_type> {
                    let receiver = {
                        #receiver_expr
                    };
                    ::snec::Handle::new(&mut self.#field_ident, receiver)
                }
            }
        };
        impls.push(token_stream);
    }
    for entry_data in requested_generated_entries {
        let entry_name = entry_data.marker_name;
        let field_ident = entry_data.field_name;
        let data_type = entry_data.field_type;
        let documentation = format!(
            "The entry identifier type for the `{}` field in the `{}` config table.",
            &field_ident,
            &struct_input.ident,
        );
        let documentation = Lit::Str(
            LitStr::new(&documentation, Span::call_site()),
        );
        let field_name_literal = Lit::Str(
            LitStr::new(&field_ident.to_string(), Span::call_site()),
        );
        let entry = quote! {
            #[doc = #documentation]
            pub enum #entry_name {}
        };
        let entry_impl = quote! {
            impl ::snec::Entry for #entry_module::#entry_name {
                type Data = #data_type;
                const NAME: &'static str = #field_name_literal;
            }
        };
        generated_entries.push(entry);
        impls.push(entry_impl);
    }
    let result = quote! {
        #(#entry_module_attributes)*
        #entry_module_visibility mod #entry_module {
            #(#generated_entries)*
        }
        #(#impls)*
    };
    println!("{}", &result);
    Ok(result)
}

/// Data needed to collect from attributes to generate one `Get` implementation for one field.
struct RequestedGetImpl {
    field_name: Ident,
    receiver_type: Type,
    receiver_expr: TokenStream,
    marker_path: Path,
}
/// Data needed to collect from attributes to generate one marker type implementing `Entry` for one field.
struct RequestedGeneratedEntry {
    field_name: Ident,
    field_type: Type,
    marker_name: Ident,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic() {
        let input = quote! {
            struct MyConfigTable {
                #[snec]
                field: String,
            }
        };
        let expected_output = quote! {
            mod entries {
                use super::*;
                #[doc = "The entry identifier type for the `field` field in the `MyConfigTable` config table."]
                pub enum Field {}
                impl ::snec::Entry for Field {
                    type Data = String;
                    const NAME: &'static str = "field";
                }
            };
            impl ::snec::Get<entries::Field> for MyConfigTable {
                type Receiver = ::snec::EmptyReceiver;
                #[inline(always)]
                fn get_ref(&self) -> &<entries::Field as ::snec::Entry>::Data {
                    &self.field
                }
                #[inline]
                fn get_handle(
                    &mut self
                ) -> ::snec::Handle<'_, entries::Field, ::snec::EmptyReceiver> {
                    let receiver = {
                        ::snec::EmptyReceiver
                    };
                    ::snec::Handle::new(&mut self.field, receiver)
                }
            }
        };
        let output = derive_config_table_expand(input).unwrap();
        assert_eq!(output.to_string(), expected_output.to_string());
    }
}