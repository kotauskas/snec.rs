use syn::{
    Ident,
    token,
    Token,
    braced,
    Attribute,
    Visibility,
    Generics,
    Field,
    punctuated::Punctuated,
    parse::{Parse, ParseStream},
};

pub struct ConfigTableStruct {
    pub attrs: Vec<Attribute>,
    pub visibility: Visibility,
    pub struct_token: Token![struct],
    pub ident: Ident,
    pub generics: Generics,
    pub braces: token::Brace,
    pub fields: Punctuated<Field, Token![,]>,
}
impl Parse for ConfigTableStruct {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let visibility = input.parse()?;
        let struct_token = input.parse()?;
        let ident = input.parse()?;
        let generics = input.parse()?;
        let inside_braces;
        let braces = braced!(inside_braces in input);
        let fields = inside_braces.call(
            |input| Punctuated::parse_terminated_with(input, Field::parse_named),
        )?;
        Ok (
            Self {attrs, visibility, struct_token, ident, generics, braces, fields}
        )
    }
}