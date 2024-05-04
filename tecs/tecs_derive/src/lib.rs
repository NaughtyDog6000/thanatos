use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, FieldsNamed};

#[proc_macro_derive(Archetype)]
pub fn derive_answer_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }), 
        ..
    }) = input.data
    else {
        panic!()
    };

    let ident = input.ident;
    let idents = named.iter().map(|field| field.ident.as_ref().unwrap());
    let types = named.iter().map(|field| &field.ty);

    let expanded = quote! {
        impl_archetype!(#ident #(#idents #types)*);
    };

    TokenStream::from(expanded)
}
