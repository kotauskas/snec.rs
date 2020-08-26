use fehler::{throw, throws};
use syn::{
    Data as TypeData, DataStruct, DataEnum, DataUnion,
    Fields, FieldsNamed,
    Meta, NestedMeta,
    Lit, LitStr,
    Path,
};
use proc_macro2::Span;

//─────────────────┐
// Match functions |
//─────────────────┘
#[throws(syn::Error)]
pub fn typedata_struct(type_data: TypeData) -> DataStruct {
    match type_data {
        TypeData::Struct(x) => x,
        TypeData::Enum(DataEnum {enum_token, ..}) => throw!(syn::Error::new(
            enum_token.span,
            "cannot use an enumeration as a configuration table",
        )),
        TypeData::Union(DataUnion {union_token, ..}) => throw!(syn::Error::new(
            union_token.span,
            "cannot use a union as a configuration table",
        )),
    }
}
#[throws(syn::Error)]
pub fn fields_named(fields: Fields) -> FieldsNamed {
    match fields {
        Fields::Named(x) => x,
        Fields::Unnamed(..) => throw!(syn::Error::new(
            Span::call_site(),
            "cannot use a tuple struct as a configuration table",
        )),
        Fields::Unit => throw!(syn::Error::new(
            Span::call_site(),
            "cannot use a unit struct as a configuration table",
        )),
    }
}
#[throws(syn::Error)]
pub fn meta_path(meta: Meta) -> Path {
    match meta {
        Meta::Path(x) => x,
        Meta::List(x) => throw!(
            syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
        ),
        Meta::NameValue(x) => throw!(
            syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
        )
    }
}
#[throws(syn::Error)]
pub fn nested_meta_normal(nested_meta: NestedMeta) -> Meta {
    match nested_meta {
        NestedMeta::Meta(x) => x,
        NestedMeta::Lit(x) => throw!(
            syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
        ),
    }
}
#[throws(syn::Error)]
pub fn nested_meta_literal(nested_meta: NestedMeta) -> Lit {
    match nested_meta {
        NestedMeta::Lit(x) => x,
        NestedMeta::Meta(x) => throw!(
            syn::Error::new_spanned(x, "unrecognized `#[snec(...)]` attribute syntax")
        ),
    }
}
#[throws(syn::Error)]
pub fn lit_str(lit: Lit) -> LitStr {
    match lit {
        Lit::Str(x) => x,
        other => throw!(syn::Error::new_spanned(other, "incorrect literal type")),
    }
}