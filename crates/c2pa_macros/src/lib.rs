//! # C2PA Transform Macros
//!
//! Attribute macros for automatically generating C2PA provenance-preserving transforms.
//!
//! ## Macros
//!
//! - `#[c2pa_pipeline]` - Wrap a function with automatic TransformContext management
//! - `#[c2pa_source]` - Define a provenance origin (root of the chain)
//! - `#[c2pa_transform]` - Define a provenance-preserving transformation
//!
//! ## Example
//!
//! ```ignore
//! #[c2pa_pipeline(generator = "demo")]
//! fn main() -> Result<(), TransformError> {
//!     let start = start_c2pa()?;
//!     let result = add_ten_c2pa(&double_c2pa(&start)?)?;
//!     println!("{}", result.payload());
//!     Ok(())
//! }
//!
//! #[c2pa_source(signer = TestSigner)]
//! fn start() -> u32 { 5 }
//!
//! #[c2pa_transform(name = "double")]
//! fn double(x: &u32) -> u32 { x * 2 }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    Error, Expr, FnArg, Ident, ItemFn, Lit, Meta, Result, ReturnType, Token, Type,
};

// ============================================================================
// #[c2pa_pipeline] - Automatic TransformContext management
// ============================================================================

struct PipelineAttr {
    generator: String,
}

impl Parse for PipelineAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut generator = String::from("c2pa_pipeline");

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for meta in metas {
            if let Meta::NameValue(nv) = &meta {
                let ident = nv.path.get_ident().ok_or_else(|| {
                    Error::new(nv.path.span(), "expected identifier")
                })?;
                if ident == "generator" {
                    if let Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                        generator = s.value();
                    }
                }
            }
        }

        Ok(PipelineAttr { generator })
    }
}

#[proc_macro_attribute]
pub fn c2pa_pipeline(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as PipelineAttr);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;
    let generator = &attr.generator;

    let output = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            c2pa_primitives::with_new_ctx(#generator, || {
                #fn_block
            })
        }
    };

    output.into()
}

// ============================================================================
// #[c2pa_source] - Verified origin point
// ============================================================================

struct SourceAttr {
    signer: Option<Expr>,
}

impl Parse for SourceAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut signer = None;

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for meta in metas {
            if let Meta::NameValue(nv) = &meta {
                let ident = nv.path.get_ident().ok_or_else(|| {
                    Error::new(nv.path.span(), "expected identifier")
                })?;
                if ident == "signer" {
                    signer = Some(nv.value.clone());
                }
            }
        }

        Ok(SourceAttr { signer })
    }
}

#[proc_macro_attribute]
pub fn c2pa_source(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as SourceAttr);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let wrapper_name = format_ident!("{}_c2pa", fn_name);

    // Extract return type
    let output_type = match &input_fn.sig.output {
        ReturnType::Type(_, ty) => ty.as_ref().clone(),
        ReturnType::Default => {
            return Error::new(input_fn.sig.span(), "c2pa_source requires a return type")
                .to_compile_error()
                .into();
        }
    };

    // Default signer
    let signer_expr = attr.signer.unwrap_or_else(|| {
        syn::parse_quote!(c2pa_primitives::TestSigner)
    });

    let output = quote! {
        // Original function
        #input_fn

        // Generated wrapper
        #fn_vis fn #wrapper_name() -> ::core::result::Result<
            c2pa_primitives::C2pa<#output_type, c2pa_primitives::Verified>,
            c2pa_primitives::TransformError
        > {
            let payload = #fn_name();
            c2pa_primitives::with_ctx(|ctx| {
                c2pa_primitives::C2paBuilder::new(payload)
                    .generator(&ctx.generator)
                    .sign(&#signer_expr)
            })
        }
    };

    output.into()
}

// ============================================================================
// #[c2pa_transform] - Provenance-preserving transformation
// ============================================================================

struct C2paTransformAttr {
    name: String,
    relationship: String,
    record_params: Vec<Ident>,
}

impl Parse for C2paTransformAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut relationship = String::from("derivedFrom");
        let mut record_params = Vec::new();

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in metas {
            match &meta {
                Meta::NameValue(nv) => {
                    let ident = nv.path.get_ident().ok_or_else(|| {
                        Error::new(nv.path.span(), "expected identifier")
                    })?;

                    match ident.to_string().as_str() {
                        "name" => {
                            if let Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                                name = Some(s.value());
                            } else {
                                return Err(Error::new(nv.value.span(), "expected string literal"));
                            }
                        }
                        "relationship" => {
                            if let Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                                relationship = s.value();
                            } else {
                                return Err(Error::new(nv.value.span(), "expected string literal"));
                            }
                        }
                        other => {
                            return Err(Error::new(ident.span(), format!("unknown attribute: {}", other)));
                        }
                    }
                }
                Meta::List(list) => {
                    let ident = list.path.get_ident().ok_or_else(|| {
                        Error::new(list.path.span(), "expected identifier")
                    })?;

                    if ident == "record" {
                        let inner: RecordAttr = syn::parse2(list.tokens.clone())?;
                        record_params = inner.params;
                    } else {
                        return Err(Error::new(ident.span(), format!("unknown attribute: {}", ident)));
                    }
                }
                Meta::Path(path) => {
                    return Err(Error::new(path.span(), "unexpected path-only attribute"));
                }
            }
        }

        let name = name.ok_or_else(|| Error::new(input.span(), "missing required `name` attribute"))?;

        Ok(C2paTransformAttr {
            name,
            relationship,
            record_params,
        })
    }
}

struct RecordAttr {
    params: Vec<Ident>,
}

impl Parse for RecordAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "params" {
            return Err(Error::new(ident.span(), "expected `params`"));
        }

        let content;
        syn::parenthesized!(content in input);
        let params = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;

        Ok(RecordAttr {
            params: params.into_iter().collect(),
        })
    }
}

fn extract_ref_type(ty: &Type) -> Option<&Type> {
    if let Type::Reference(type_ref) = ty {
        Some(&type_ref.elem)
    } else {
        None
    }
}

fn extract_result_inner(ty: &Type) -> Option<(&Type, &Type)> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() == 2 {
                        if let (syn::GenericArgument::Type(ok_ty), syn::GenericArgument::Type(err_ty)) =
                            (&args.args[0], &args.args[1])
                        {
                            return Some((ok_ty, err_ty));
                        }
                    }
                }
            }
        }
    }
    None
}

fn relationship_to_tokens(rel: &str) -> TokenStream2 {
    match rel {
        "parentOf" => quote! { c2pa_primitives::IngredientRelation::ParentOf },
        "componentOf" => quote! { c2pa_primitives::IngredientRelation::ComponentOf },
        "inputTo" => quote! { c2pa_primitives::IngredientRelation::InputTo },
        "derivedFrom" => quote! { c2pa_primitives::IngredientRelation::DerivedFrom },
        "composedFrom" => quote! { c2pa_primitives::IngredientRelation::ComposedFrom },
        _ => quote! { c2pa_primitives::IngredientRelation::DerivedFrom },
    }
}

fn generate_commit_code(param_name: &Ident) -> TokenStream2 {
    quote! {
        {
            use ::sha2::{Sha256, Digest};
            let bytes = format!("{:?}", &#param_name);
            let mut hasher = Sha256::new();
            hasher.update(bytes.as_bytes());
            let hash: [u8; 32] = hasher.finalize().into();
            (stringify!(#param_name).to_string(), hash)
        }
    }
}

#[proc_macro_attribute]
pub fn c2pa_transform(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as C2paTransformAttr);
    let input_fn = parse_macro_input!(item as ItemFn);

    match generate_transform(&attr, &input_fn) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_transform(attr: &C2paTransformAttr, input_fn: &ItemFn) -> Result<TokenStream2> {
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let wrapper_name = format_ident!("{}_c2pa", fn_name);

    // Extract function arguments
    let args: Vec<_> = input_fn.sig.inputs.iter().collect();
    if args.is_empty() {
        return Err(Error::new(
            input_fn.sig.span(),
            "c2pa_transform requires at least one argument (the input reference)",
        ));
    }

    // First argument must be a reference type `&T`
    let first_arg = match &args[0] {
        FnArg::Typed(pat_type) => pat_type,
        FnArg::Receiver(_) => {
            return Err(Error::new(args[0].span(), "self receivers are not supported"));
        }
    };

    let input_inner_type = extract_ref_type(&first_arg.ty).ok_or_else(|| {
        Error::new(first_arg.ty.span(), "first argument must be a reference type (&T)")
    })?;

    let first_arg_pat = &first_arg.pat;

    // Additional parameters (starting from index 1)
    let extra_params: Vec<_> = args
        .iter()
        .skip(1)
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                Some(pat_type)
            } else {
                None
            }
        })
        .collect();

    // Extract return type
    let output_type = match &input_fn.sig.output {
        ReturnType::Type(_, ty) => ty.as_ref(),
        ReturnType::Default => {
            return Err(Error::new(
                input_fn.sig.span(),
                "c2pa_transform requires a return type",
            ));
        }
    };

    // Check if it's a Result type (fallible function)
    let (actual_output_type, is_fallible) = if let Some((ok_ty, _err_ty)) = extract_result_inner(output_type) {
        (ok_ty.clone(), true)
    } else {
        (output_type.clone(), false)
    };

    // Generate parameter declarations for wrapper function (no ctx!)
    let wrapper_params: Vec<TokenStream2> = extra_params
        .iter()
        .map(|param| {
            let pat = &param.pat;
            let ty = &param.ty;
            quote! { #pat: #ty }
        })
        .collect();

    // Generate parameter passing for the original function call
    let param_pass: Vec<TokenStream2> = extra_params
        .iter()
        .map(|param| {
            let pat = &param.pat;
            quote! { #pat }
        })
        .collect();

    // Generate commit collection for recorded parameters
    let commit_code: Vec<TokenStream2> = attr
        .record_params
        .iter()
        .map(|param_name| generate_commit_code(param_name))
        .collect();

    let has_commits = !commit_code.is_empty();
    let transform_name = &attr.name;
    let relationship = relationship_to_tokens(&attr.relationship);

    // Generate the original function call
    let call_original = if param_pass.is_empty() {
        quote! { #fn_name(#first_arg_pat) }
    } else {
        quote! { #fn_name(#first_arg_pat, #(#param_pass),*) }
    };

    // Generate payload extraction (handle fallible vs infallible)
    let payload_extraction = if is_fallible {
        quote! {
            let out_payload = #call_original
                .map_err(|e| c2pa_primitives::TransformError::C2pa(format!("{:?}", e)))?;
        }
    } else {
        quote! {
            let out_payload = #call_original;
        }
    };

    // Generate commits collection
    let commits_collection = if has_commits {
        quote! {
            let mut param_commits: Vec<(String, [u8; 32])> = Vec::new();
            #(
                param_commits.push(#commit_code);
            )*
        }
    } else {
        quote! {
            let param_commits: Vec<(String, [u8; 32])> = Vec::new();
        }
    };

    // Generate wrapper function signature (NO ctx argument!)
    let wrapper_signature = if wrapper_params.is_empty() {
        quote! {
            #fn_vis fn #wrapper_name(
                input: &c2pa_primitives::C2pa<#input_inner_type, c2pa_primitives::Verified>,
            ) -> ::core::result::Result<c2pa_primitives::C2pa<#actual_output_type, c2pa_primitives::Verified>, c2pa_primitives::TransformError>
        }
    } else {
        quote! {
            #fn_vis fn #wrapper_name(
                input: &c2pa_primitives::C2pa<#input_inner_type, c2pa_primitives::Verified>,
                #(#wrapper_params,)*
            ) -> ::core::result::Result<c2pa_primitives::C2pa<#actual_output_type, c2pa_primitives::Verified>, c2pa_primitives::TransformError>
        }
    };

    // Generate the complete output
    let output = quote! {
        // Original function (unchanged)
        #input_fn

        // Generated wrapper function (uses thread-local ctx)
        #wrapper_signature {
            // Collect parameter commits BEFORE calling original function
            #commits_collection

            // Extract payload from verified input
            let #first_arg_pat = input.payload();

            // Call the original function
            #payload_extraction

            // Build the provenance-aware result using thread-local context
            c2pa_primitives::with_ctx(|ctx| {
                c2pa_primitives::transform_helper::build_transform_result(
                    out_payload,
                    input,
                    #transform_name,
                    #relationship,
                    param_commits,
                    ctx,
                )
            })
        }
    };

    Ok(output)
}
