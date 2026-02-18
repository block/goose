use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ImplItem, ItemImpl, Lit, Pat, ReturnType, Type};

/// Marks an impl block as containing `#[custom_method("...")]`-annotated handlers.
///
/// Generates a `handle_custom_request` dispatcher that:
/// - Prefixes each method name with `_goose/`
/// - Parses JSON params into the handler's typed parameter (if any)
/// - Serializes the handler's return value to JSON
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

                // Determine if the method takes a typed parameter (beyond &self).
                let param_type = extract_param_type(&method.sig);
                let return_type = extract_return_type(&method.sig);

                routes.push(Route {
                    method_name: name,
                    fn_ident,
                    param_type,
                    return_type,
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
                    // Handler takes a typed param: parse from JSON, call, serialize result.
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
                    // Handler takes no params: call directly, serialize result.
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

    // Append the generated dispatcher to the impl block.
    let dispatcher_item: ImplItem =
        syn::parse2(dispatcher).expect("generated dispatcher must parse");
    impl_block.items.push(dispatcher_item);

    TokenStream::from(quote! { #impl_block })
}

struct Route {
    method_name: String,
    fn_ident: syn::Ident,
    param_type: Option<Type>,
    #[allow(dead_code)]
    return_type: Option<Type>,
}

/// Extract the type of the first non-self parameter, if any.
fn extract_param_type(sig: &syn::Signature) -> Option<Type> {
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            // Skip if it's self
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

/// Extract the Ok type from Result<T, E>, if the return type is a Result.
fn extract_return_type(sig: &syn::Signature) -> Option<Type> {
    if let ReturnType::Type(_, ty) = &sig.output {
        Some((**ty).clone())
    } else {
        None
    }
}
