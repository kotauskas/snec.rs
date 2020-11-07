use super::*;

/// Converts an iterator over normal Rust attributes to an iterator of `SnecAttribute`s by filtering out all attributes which were not for Snec.
#[inline]
pub fn filter_to_snec_attributes(
    attributes: impl IntoIterator<Item = Attribute>,
) -> impl Iterator<Item = SnecAttribute> {
    attributes.into_iter().filter_map(|attr| {
        SnecAttribute::try_from(attr).ok()
    })
}

pub fn concat_to_path(x: Ident, y: Ident) -> Path {
    let x = PathSegment {
        ident: x,
        arguments: PathArguments::None,
    };
    let y = PathSegment {
        ident: y,
        arguments: PathArguments::None,
    };
    let segments = {
        let mut segments = Punctuated::new();
        segments.push(x);
        segments.push(y);
        segments
    };
    Path {leading_colon: None, segments}
}

/// Constructs an expression which points to the `::snec::EmptyReceiver` unit constructor with call-site hygeine.
#[inline]
pub fn default_receiver_expr() -> Expr {
    let expr = ExprPath {
        attrs: Vec::new(),
        qself: None,
        path: default_receiver_path(),
    };
    Expr::Path(expr)
}
/// Constructs a type which points to `::snec::EmptyReceiver` with call-site hygeine.
pub fn default_receiver_type() -> Type {
    let ty = TypePath {
        qself: None,
        path: default_receiver_path(),
    };
    Type::Path(ty)
}
#[inline]
fn default_receiver_path() -> Path {
    let leading_colon = Some(
        token::Colon2 {
            spans: [Span::call_site(); 2]
        }
    );
    let to_path_segment = |ident| PathSegment {ident, arguments: PathArguments::None};
    let snec_segment = to_path_segment(
        Ident::new("snec", Span::call_site())
    );
    let emptyreceiver_segment = to_path_segment(
        Ident::new("EmptyReceiver", Span::call_site())
    );
    let segments = {
        let mut segments = Punctuated::new();
        segments.push(snec_segment);
        segments.push(emptyreceiver_segment);
        segments
    };
    Path {leading_colon, segments}
}
/// Constructs an identifier pointing to `entries` with call-site hygeine.
#[inline]
pub fn default_entry_module() -> Ident {
    Ident::new("entries", Span::call_site())
}

/// Converts a `snake_case` identifier to a `CamelCase` one, preserving its exact span.
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