use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ItemStruct, LitStr};

struct RouteArgs {
    method: syn::Ident,
    path: LitStr,
}

impl syn::parse::Parse for RouteArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let method = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let path = input.parse()?;
        Ok(Self { method, path })
    }
}

struct ControllerArgs {
    prefix: LitStr,
}

impl syn::parse::Parse for ControllerArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            prefix: input.parse()?,
        })
    }
}

fn derive_clone_with_impls(input: &ItemStruct, extra: TokenStream2) -> TokenStream2 {
    quote! {
        #[derive(Clone)]
        #input
        #extra
    }
}

fn axum_routing_call(method: &syn::Ident, fn_ident: &syn::Ident) -> TokenStream2 {
    match method.to_string().to_uppercase().as_str() {
        "GET" => quote! { axum::routing::get(Self::#fn_ident) },
        "POST" => quote! { axum::routing::post(Self::#fn_ident) },
        "PUT" => quote! { axum::routing::put(Self::#fn_ident) },
        "DELETE" => quote! { axum::routing::delete(Self::#fn_ident) },
        "PATCH" => quote! { axum::routing::patch(Self::#fn_ident) },
        method => panic!("Unsupported HTTP method: {method}"),
    }
}

#[proc_macro_attribute]
pub fn injectable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    derive_clone_with_impls(
        &input,
        quote! {
            impl ::fragment_rs::core::container::Injectable for #name {
                fn scope() -> ::fragment_rs::core::container::Scope {
                    ::fragment_rs::core::container::Scope::Singleton
                }
            }
            impl ::fragment_rs::core::container::Service for #name {}
        },
    )
    .into()
}

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    derive_clone_with_impls(
        &input,
        quote! {
            impl ::fragment_rs::core::container::Service for #name {}
        },
    )
    .into()
}

#[proc_macro_attribute]
pub fn exception_filter(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    derive_clone_with_impls(
        &input,
        quote! {
            impl ::fragment_rs::core::filter::ExceptionFilter for #name {}
            impl ::fragment_rs::core::container::Service for #name {}
        },
    )
    .into()
}

#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ControllerArgs);
    let prefix = args.prefix.value();
    let prefix = prefix.trim_end_matches('/');

    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let self_ty = impl_block.self_ty.clone();
    let mut routes: Vec<TokenStream2> = Vec::new();

    for item in &mut impl_block.items {
        if let syn::ImplItem::Fn(method) = item {
            let fn_ident = &method.sig.ident;
            let (route_attrs, other_attrs): (Vec<_>, Vec<_>) = std::mem::take(&mut method.attrs)
                .into_iter()
                .partition(|attr| attr.path().is_ident("route"));
            method.attrs = other_attrs;

            for attr in route_attrs {
                match attr.parse_args::<RouteArgs>() {
                    Ok(RouteArgs {
                        method: http_method,
                        path,
                    }) => {
                        let sub = path.value();
                        let sub = sub.trim_start_matches('/');
                        let full_path = if prefix.is_empty() {
                            format!("/{sub}")
                        } else {
                            format!("{prefix}/{sub}")
                        };
                        let routing_call = axum_routing_call(&http_method, fn_ident);
                        routes.push(quote! {
                            router = router.route(#full_path, #routing_call);
                        });
                    }
                    Err(error) => return error.to_compile_error().into(),
                }
            }
        }
    }

    quote! {
        #impl_block

        impl ::fragment_rs::core::routing::Controller for #self_ty {
            fn register_routes(mut router: axum::Router) -> axum::Router {
                #(#routes)*
                router
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_derive(FragmentPrivacy, attributes(pii))]
pub fn derive_fragment_privacy(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    quote! {
        impl ::fragment_rs::core::privacy::Privacy for #name {
            fn masked(&self) -> serde_json::Value {
                serde_json::to_value(self).unwrap_or_default()
            }
        }
    }
    .into()
}
