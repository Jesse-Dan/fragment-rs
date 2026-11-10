use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ItemStruct, LitStr};

struct RouteArgs {
    method: syn::Path,
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

/// Maps a method path onto the matching Axum routing call.
///
/// The last segment of the path carries the method name, so both `GET` and
/// `fragment_rs::GET` are accepted.
fn axum_routing_call(method: &syn::Path, fn_ident: &syn::Ident) -> TokenStream2 {
    let name = method
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default();
    match name.to_uppercase().as_str() {
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

    // A service that derives `Default` can be built by the framework, so it
    // registers itself. One that does not has to be registered by hand.
    let derives_default = input.attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }
        attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated,
        )
        .map(|idents| idents.iter().any(|ident| ident == "Default"))
        .unwrap_or(false)
    });

    let registration = if derives_default {
        quote! {
            ::fragment_rs::core::container::inventory::submit! {
                ::fragment_rs::core::container::ServiceRegistration {
                    create: ::fragment_rs::core::container::boxed::<#name>,
                }
            }
        }
    } else {
        quote! {}
    };

    derive_clone_with_impls(
        &input,
        quote! {
            impl ::fragment_rs::core::container::Service for #name {}

            #registration
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
    let mut methods: Vec<TokenStream2> = Vec::new();

    for item in &mut impl_block.items {
        if let syn::ImplItem::Fn(method) = item {
            let fn_ident = &method.sig.ident;
            // The `#[route(...)]` attributes are read here to build the router
            // but are deliberately left in place. Keeping them means the
            // `route` macro and the HTTP method constants stay referenced, so
            // the imports a controller writes are not reported as unused.
            let route_attrs: Vec<syn::Attribute> = method
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("route"))
                .cloned()
                .collect();

            for attr in route_attrs {
                match attr.parse_args::<RouteArgs>() {
                    Ok(RouteArgs {
                        method: http_method,
                        path,
                    }) => {
                        let sub = path.value();
                        let sub = sub.trim_matches('/');
                        // A route of "/" or "" addresses the prefix itself. Joining
                        // it naively would produce a trailing slash, and Axum
                        // matches paths exactly, so "/api/customers/" would not
                        // answer a request for "/api/customers".
                        let full_path = match (prefix.is_empty(), sub.is_empty()) {
                            (true, true) => "/".to_owned(),
                            (true, false) => format!("/{sub}"),
                            (false, true) => prefix.to_owned(),
                            (false, false) => format!("{prefix}/{sub}"),
                        };
                        let routing_call = axum_routing_call(&http_method, fn_ident);
                        routes.push(quote! {
                            router = router.route(#full_path, #routing_call);
                        });
                        // The method constant is referenced here so that the
                        // import a controller writes for it is not reported as
                        // unused: the constant only appears inside an
                        // attribute, which the compiler never resolves.
                        methods.push(quote! {
                            let _ = #http_method;
                        });
                    }
                    Err(error) => return error.to_compile_error().into(),
                }
            }
        }
    }

    quote! {
        #impl_block

        impl #self_ty {
            /// Registers this controller's routes on a router.
            ///
            /// Pass it to `FragmentApp::controller` to serve the routes.
            pub fn register(mut router: axum::Router) -> axum::Router {
                #(#methods)*
                #(#routes)*
                router
            }
        }

        impl ::fragment_rs::core::routing::Controller for #self_ty {
            fn register_routes(router: axum::Router) -> axum::Router {
                Self::register(router)
            }
        }

        // The controller registers itself, so the application finds it
        // without being told about it.
        ::fragment_rs::core::routing::inventory::submit! {
            ::fragment_rs::core::routing::ControllerRegistration {
                register: <#self_ty>::register,
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
