use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, FnArg, GenericArgument, ImplItem, ItemImpl, Lit, Pat, PathArguments,
    ReturnType, Type,
};

/// Marks an impl block as containing `#[custom_method("...")]`-annotated handlers.
///
/// Generates two methods on the impl:
///
/// 1. `handle_custom_request` — a dispatcher that:
///    - Prefixes each method name with `_goose/`
///    - Parses JSON params into the handler's typed parameter (if any)
///    - Serializes the handler's return value to JSON
///
/// 2. `custom_method_schemas` — returns a `Vec<CustomMethodSchema>` with
///    JSON Schema for each method's params and response types. Types that
///    implement `schemars::JsonSchema` get a full schema; `serde_json::Value`
///    params/responses produce `None`.
///
/// # Handler signatures
///
/// Handlers may take zero or one parameter (beyond `&self`):
///
/// ```ignore
/// // No params — called for requests with no/empty params
/// #[custom_method("session/list")]
/// async fn on_list_sessions(&self) -> Result<ListSessionsResponse, sacp::Error> { .. }
///
/// // Typed params — JSON params auto-deserialized
/// #[custom_method("session/get")]
/// async fn on_get_session(&self, req: GetSessionRequest) -> Result<GetSessionResponse, sacp::Error> { .. }
/// ```
///
/// The return type must be `Result<T, sacp::Error>` where `T: Serialize`.
#[proc_macro_attribute]
pub fn custom_methods(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);

    let mut routes: Vec<Route> = Vec::new();

    // Collect all #[custom_method("...")] annotations and strip them.
    for item in &mut impl_block.items {
        if let ImplItem::Fn(method) = item {
            let mut route_name = None;
            method.attrs.retain(|attr| {
                if attr.path().is_ident("custom_method") {
                    if let Ok(meta_list) = attr.meta.require_list() {
                        if let Ok(Lit::Str(s)) = meta_list.parse_args::<Lit>() {
                            route_name = Some(s.value());
                        }
                    }
                    false // strip the attribute
                } else {
                    true // keep other attributes
                }
            });

            if let Some(name) = route_name {
                let fn_ident = method.sig.ident.clone();

                let param_type = extract_param_type(&method.sig);
                let return_type = extract_return_type(&method.sig);
                let ok_type = extract_result_ok_type(&method.sig);

                routes.push(Route {
                    method_name: name,
                    fn_ident,
                    param_type,
                    return_type,
                    ok_type,
                });
            }
        }
    }

    // Generate the dispatch arms.
    let arms: Vec<_> = routes
        .iter()
        .map(|route| {
            let full_method = format!("_goose/{}", route.method_name);
            let fn_ident = &route.fn_ident;

            match &route.param_type {
                Some(_) => {
                    quote! {
                        #full_method => {
                            let req = serde_json::from_value(params)
                                .map_err(|e| sacp::Error::invalid_params().data(e.to_string()))?;
                            let result = self.#fn_ident(req).await?;
                            serde_json::to_value(&result)
                                .map_err(|e| sacp::Error::internal_error().data(e.to_string()))
                        }
                    }
                }
                None => {
                    quote! {
                        #full_method => {
                            let result = self.#fn_ident().await?;
                            serde_json::to_value(&result)
                                .map_err(|e| sacp::Error::internal_error().data(e.to_string()))
                        }
                    }
                }
            }
        })
        .collect();

    // Generate schema entries for each route using SchemaGenerator for $ref dedup.
    let schema_entries: Vec<_> = routes
        .iter()
        .map(|route| {
            let full_method = format!("_goose/{}", route.method_name);

            let params_expr = if let Some(pt) = &route.param_type {
                if is_json_value(pt) {
                    quote! { None }
                } else {
                    quote! { Some(generator.subschema_for::<#pt>()) }
                }
            } else {
                quote! { None }
            };

            let response_expr = if let Some(ok_ty) = &route.ok_type {
                if is_json_value(ok_ty) {
                    quote! { None }
                } else {
                    quote! { Some(generator.subschema_for::<#ok_ty>()) }
                }
            } else {
                quote! { None }
            };

            let params_name_expr = if let Some(pt) = &route.param_type {
                if is_json_value(pt) {
                    quote! { None }
                } else {
                    let name = type_name(pt);
                    quote! { Some(#name.to_string()) }
                }
            } else {
                quote! { None }
            };

            let response_name_expr = if let Some(ok_ty) = &route.ok_type {
                if is_json_value(ok_ty) {
                    quote! { None }
                } else {
                    let name = type_name(ok_ty);
                    quote! { Some(#name.to_string()) }
                }
            } else {
                quote! { None }
            };

            quote! {
                crate::custom_requests::CustomMethodSchema {
                    method: #full_method.to_string(),
                    params_schema: #params_expr,
                    params_type_name: #params_name_expr,
                    response_schema: #response_expr,
                    response_type_name: #response_name_expr,
                }
            }
        })
        .collect();

    // Generate the handle_custom_request method.
    let dispatcher = quote! {
        async fn handle_custom_request(
            &self,
            method: &str,
            params: serde_json::Value,
        ) -> Result<serde_json::Value, sacp::Error> {
            match method {
                #(#arms)*
                _ => Err(sacp::Error::method_not_found()),
            }
        }
    };

    // Generate the custom_method_schemas method.
    let schemas_fn = quote! {
        pub fn custom_method_schemas(generator: &mut schemars::SchemaGenerator) -> Vec<crate::custom_requests::CustomMethodSchema> {
            vec![
                #(#schema_entries),*
            ]
        }
    };

    // Append the generated methods to the impl block.
    let dispatcher_item: ImplItem =
        syn::parse2(dispatcher).expect("generated dispatcher must parse");
    impl_block.items.push(dispatcher_item);

    let schemas_item: ImplItem = syn::parse2(schemas_fn).expect("generated schemas fn must parse");
    impl_block.items.push(schemas_item);

    TokenStream::from(quote! { #impl_block })
}

struct Route {
    method_name: String,
    fn_ident: syn::Ident,
    param_type: Option<Type>,
    #[allow(dead_code)]
    return_type: Option<Type>,
    ok_type: Option<Type>,
}

/// Extract the type of the first non-self parameter, if any.
fn extract_param_type(sig: &syn::Signature) -> Option<Type> {
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                if pat_ident.ident == "self" {
                    continue;
                }
            }
            return Some((*pat_type.ty).clone());
        }
    }
    None
}

/// Extract the full return type (e.g. `Result<T, E>`).
fn extract_return_type(sig: &syn::Signature) -> Option<Type> {
    if let ReturnType::Type(_, ty) = &sig.output {
        Some((**ty).clone())
    } else {
        None
    }
}

/// Extract `T` from `Result<T, E>` in the return type.
fn extract_result_ok_type(sig: &syn::Signature) -> Option<Type> {
    let ty = match &sig.output {
        ReturnType::Type(_, ty) => ty,
        _ => return None,
    };

    // Peel through the type to find a path ending in `Result`.
    if let Type::Path(type_path) = ty.as_ref() {
        let last_seg = type_path.path.segments.last()?;
        if last_seg.ident == "Result" {
            if let PathArguments::AngleBracketed(args) = &last_seg.arguments {
                // First generic argument is the Ok type.
                if let Some(GenericArgument::Type(ok_ty)) = args.args.first() {
                    return Some(ok_ty.clone());
                }
            }
        }
    }
    None
}

/// Extract the last segment name from a type path (e.g. `GetSessionRequest` from
/// `crate::custom_requests::GetSessionRequest` or just `GetSessionRequest`).
fn type_name(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    quote::quote!(#ty).to_string()
}

/// Check if a type is `serde_json::Value` (matches `Value` or `serde_json::Value`).
fn is_json_value(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        let segments: Vec<_> = type_path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let strs: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();
        matches!(strs.as_slice(), ["serde_json", "Value"] | ["Value"])
    } else {
        false
    }
}
