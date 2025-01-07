use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, punctuated::Punctuated, Expr, ExprLit,
    FnArg, ItemFn, Lit, Meta, Pat, PatType, Token,
};

struct MacroArgs {
    name: Option<String>,
    description: Option<String>,
    param_descriptions: HashMap<String, String>,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut param_descriptions = HashMap::new();

        let meta_list: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;

        for meta in meta_list {
            match meta {
                Meta::NameValue(nv) => {
                    let ident = nv.path.get_ident().unwrap().to_string();
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }) = nv.value
                    {
                        match ident.as_str() {
                            "name" => name = Some(lit_str.value()),
                            "description" => description = Some(lit_str.value()),
                            _ => {}
                        }
                    }
                }
                Meta::List(list) if list.path.is_ident("params") => {
                    let nested: Punctuated<Meta, Token![,]> =
                        list.parse_args_with(Punctuated::parse_terminated)?;

                    for meta in nested {
                        if let Meta::NameValue(nv) = meta {
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit_str),
                                ..
                            }) = nv.value
                            {
                                let param_name = nv.path.get_ident().unwrap().to_string();
                                param_descriptions.insert(param_name, lit_str.value());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(MacroArgs {
            name,
            description,
            param_descriptions,
        })
    }
}

#[proc_macro_attribute]
pub fn tool(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as MacroArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    // Extract function details
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    // Generate PascalCase struct name from the function name
    let struct_name = format_ident!("{}", { fn_name_str.to_case(Case::Pascal) });

    // Use provided name or function name as default
    let tool_name = args.name.unwrap_or(fn_name_str);
    let tool_description = args.description.unwrap_or_default();

    // Extract parameter names, types, and descriptions
    let mut param_defs = Vec::new();
    let mut param_names = Vec::new();

    for arg in input_fn.sig.inputs.iter() {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(param_ident) = &**pat {
                let param_name = &param_ident.ident;
                let param_name_str = param_name.to_string();
                let description = args
                    .param_descriptions
                    .get(&param_name_str)
                    .map(|s| s.as_str())
                    .unwrap_or("");

                param_names.push(param_name);
                param_defs.push(quote! {
                    #[schemars(description = #description)]
                    #param_name: #ty
                });
            }
        }
    }

    // Generate the implementation
    let params_struct_name = format_ident!("{}Parameters", struct_name);
    let expanded = quote! {
        #[derive(serde::Deserialize, schemars::JsonSchema)]
        struct #params_struct_name {
            #(#param_defs,)*
        }

        #input_fn

        #[derive(Default)]
        struct #struct_name;

        #[async_trait::async_trait]
        impl mcp_core::handler::ToolHandler for #struct_name {
            fn name(&self) -> &'static str {
                #tool_name
            }

            fn description(&self) -> &'static str {
                #tool_description
            }

            fn schema(&self) -> serde_json::Value {
                mcp_core::handler::generate_schema::<#params_struct_name>()
                    .expect("Failed to generate schema")
            }

            async fn call(&self, params: serde_json::Value) -> Result<serde_json::Value, mcp_core::handler::ToolError> {
                let params: #params_struct_name = serde_json::from_value(params)
                    .map_err(|e| mcp_core::handler::ToolError::InvalidParameters(e.to_string()))?;

                // Extract parameters and call the function
                let result = #fn_name(#(params.#param_names,)*).await
                    .map_err(|e| mcp_core::handler::ToolError::ExecutionError(e.to_string()))?;

                Ok(serde_json::to_value(result).expect("should serialize"))

            }
        }
    };

    TokenStream::from(expanded)
}
