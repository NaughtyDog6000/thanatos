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
    let fields = &named.iter().map(|field| field.ident.as_ref().unwrap()).collect::<Vec<_>>();
    let types = &named.iter().map(|field| &field.ty).collect::<Vec<_>>();

    let expanded = quote! {
        impl tecs::Archetype for #ident {
            fn columns() -> Vec<std::any::TypeId> {
                vec![#(std::any::TypeId::of::<#types>()),*]
            }

            fn add(self, table: &tecs::Table) -> tecs::RowIndex {
                table.length.set(table.length.get() + 1);
                let mut columns = table.columns_mut();
                #(
                    columns.next().unwrap().data.push::<#types>(self.#fields);
                )*
                tecs::RowIndex(table.length.get() as u32 - 1)
            }

            fn remove(table: &tecs::Table, row: tecs::RowIndex) {
                table.length.set(table.length.get() - 1);
                let mut columns = table.columns_mut();
                #(
                    columns.next().unwrap().data.run::<#types>(|data| { data.swap_remove(row.0 as usize); });
                )*
            }

            fn get(table: &tecs::Table, row: tecs::RowIndex) -> Self {
                let mut columns = table.columns();
                Self {
                    #(#fields: columns.next().unwrap().get::<#types>(row).unwrap().clone()),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
