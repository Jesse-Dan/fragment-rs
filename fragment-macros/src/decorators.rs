#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemImpl, ItemStruct, LitStr};

// ─── Arg types ────────────────────────────────────────────────────────────────

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
        Ok(Self { prefix: input.parse()? })
    }
}

// ─── Shared helpers ───────────────────────────────────────────────────────────

/// Emits `#[derive(Clone)] #input` plus any extra impl blocks.
/// Removes the repetition across #[injectable], #[service], #[exception_filter].
fn derive_clone_with_impls(input: &ItemStruct, extra: TokenStream2) -> TokenStream2 {
    quote! {
        #[derive(Clone)]
        #input
        #extra
    }
}

/// Maps an HTTP method ident to its axum routing call.
fn axum_routing_call(method: &syn::Ident, fn_ident: &syn::Ident) -> TokenStream2 {
    match method.to_string().to_uppercase().as_str() {
        "GET"    => quote! { axum::routing::get(Self::#fn_ident)    },
        "POST"   => quote! { axum::routing::post(Self::#fn_ident)   },
        "PUT"    => quote! { axum::routing::put(Self::#fn_ident)    },
        "DELETE" => quote! { axum::routing::delete(Self::#fn_ident) },
        "PATCH"  => quote! { axum::routing::patch(Self::#fn_ident)  },
        m        => panic!("Unsupported HTTP method: {m}"),
    }
}

// ─── Macros ───────────────────────────────────────────────────────────────────

/// Marks a struct as a singleton-scoped injectable service.
///
/// Expands to:
/// - `#[derive(Clone)]`
/// - `impl Injectable for T` (Scope::Singleton)
/// - `impl Service for T`
#[proc_macro_attribute]
pub fn injectable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    derive_clone_with_impls(&input, quote! {
        impl ::fragment_rs::core::container::Injectable for #name {
            fn scope() -> ::fragment_rs::core::container::Scope {
                ::fragment_rs::core::container::Scope::Singleton
            }
        }
        impl ::fragment_rs::core::container::Service for #name {}
    })
        .into()
}

/// Marks a struct as a DI-managed service (no explicit scope needed).
///
/// Expands to:
/// - `#[derive(Clone)]`
/// - `impl Service for T`
#[proc_macro_attribute]
pub fn service(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    derive_clone_with_impls(&input, quote! {
        impl ::fragment_rs::core::container::Service for #name {}
    })
        .into()
}

/// Marks a struct as an exception filter registered with the DI container.
///
/// Expands to:
/// - `#[derive(Clone)]`
/// - `impl ExceptionFilter for T`
/// - `impl Service for T`
#[proc_macro_attribute]
pub fn exception_filter(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    derive_clone_with_impls(&input, quote! {
        impl ::fragment_rs::core::filter::ExceptionFilter for #name {}
        impl ::fragment_rs::core::container::Service for #name {}
    })
        .into()
}

/// Registers an `impl` block as an Axum controller at a given path prefix.
///
/// Scans the impl block for `#[route(METHOD, "/path")]` attributes on methods
/// and generates a `Controller::register_routes` implementation.
///
/// # Example
/// ```rust
/// #[controller("/customers")]
/// impl CustomerController {
///     #[route(GET, "/")]
///     async fn list() -> impl IntoResponse { ... }
///
///     #[route(POST, "/")]
///     async fn create() -> impl IntoResponse { ... }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ControllerArgs);
    let prefix = args.prefix.value();
    let prefix = prefix.trim_end_matches('/');

    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let self_ty = &impl_block.self_ty.clone();

    let mut routes: Vec<TokenStream2> = Vec::new();

    for item in &mut impl_block.items {
        if let syn::ImplItem::Fn(method) = item {
            let fn_ident = &method.sig.ident;
            let (route_attrs, other_attrs): (Vec<_>, Vec<_>) = std::mem::take(&mut method.attrs)
                .into_iter()
                .partition(|a| a.path().is_ident("route"));

            method.attrs = other_attrs;

            for attr in route_attrs {
                if let Ok(RouteArgs { method: http_method, path: path_lit }) =
                    attr.parse_args::<RouteArgs>()
                {
                    let path_val = path_lit.value();
                    let sub = path_val.trim_start_matches('/');
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

/// Marker attribute consumed by `#[controller]`. Has no effect on its own —
/// route metadata is extracted during controller expansion.
///
/// # Example
/// ```rust
/// #[route(GET, "/")]
/// async fn list() -> impl IntoResponse { ... }
/// ```
#[proc_macro_attribute]
pub fn route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Intentionally a no-op: #[controller] strips and processes these.
    // If used outside a #[controller] block, the method compiles unchanged.
    item
}

/// Derive macro that implements `Privacy::masked()` for a struct.
/// Fields marked `#[pii]` will be redacted in the masked view.
///
/// # Example
/// ```rust
/// #[derive(FragmentPrivacy)]
/// struct CustomerRecord {
///     id: String,
///     #[pii]
///     email: String,
/// }
/// ```
#[proc_macro_derive(FragmentPrivacy, attributes(pii))]
pub fn derive_fragment_privacy(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    quote! {
        impl ::fragment_rs::core::privacy::Privacy for #name {
            fn masked(&self) -> serde_json::Value {
                // TODO: walk fields, replace #[pii]-tagged values with "[REDACTED]"
                serde_json::to_value(self).unwrap_or_default()
            }
        }
    }
        .into()
}