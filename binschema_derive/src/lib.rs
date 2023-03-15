
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    token::Comma,
    punctuated::Punctuated,
    parse_macro_input,
    DeriveInput,
    Data,
    DataStruct,
    Fields,
    FieldsNamed,
    FieldsUnnamed,
    DataEnum,
};
use quote::quote;


fn fields_schema(fields: &Fields) -> TokenStream2 {
    match fields {
        &Fields::Named(FieldsNamed { ref named, .. }) => {
            let inner = named.iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_ty = &field.ty;
                    quote! {
                        (#field_name: %<#field_ty as ::binschema::KnownSchema>::schema())
                    }
                })
                .collect::<Punctuated<_, Comma>>();
            quote! {
                { #inner }
            }
        },
        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
            if unnamed.len() == 0 {
                quote! {
                    ()
                }
            } else if unnamed.len() == 1 {
                let inner_ty = &unnamed[0].ty;
                quote! {
                    %<#inner_ty as ::binschema::KnownSchema>::schema()
                }
            } else {
                let inner = unnamed.iter()
                    .map(|field| {
                        let field_ty = &field.ty;
                        quote! {
                            (%<#field_ty as ::binschema::KnownSchema>::schema())
                        }
                    })
                    .collect::<Punctuated<_, Comma>>();
                quote! {
                    ( #inner )
                }
            }
        },
        &Fields::Unit => {
            quote! {
                ()
            }
        },
    }
}

#[proc_macro_derive(KnownSchema)]
pub fn derive_known_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let schema = match &input.data {
        &Data::Struct(DataStruct { ref fields, .. }) => fields_schema(fields),
        &Data::Enum(DataEnum { ref variants, .. }) => {
            let inner = variants.iter()
                .map(|variant| {
                    let variant_name = &variant.ident;
                    let inner = fields_schema(&variant.fields);
                    quote! {
                        #variant_name(#inner)
                    }
                })
                .collect::<Punctuated<_, Comma>>();
            quote! {
                enum { #inner }
            }
        },
        &Data::Union(_) => panic!("cannot derive KnownSchema on a union"),
    };
    
    quote! {
        impl ::binschema::KnownSchema for #name {
            fn schema() -> ::binschema::Schema {
                ::binschema::schema!(#schema)
            }
        }
    }.into()
}
