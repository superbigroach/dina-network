use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ImplItem, ItemImpl, ItemStruct, Pat, ReturnType};

/// Attribute macro for contract structs.
///
/// Generates:
/// - A `__storage` field for internal storage bookkeeping
/// - Borsh and Serde derive implementations
/// - WASM entry points (`__init` and `__call`)
///
/// # Example
/// ```ignore
/// #[dina_contract]
/// pub struct MyToken {
///     name: String,
///     total_supply: u64,
/// }
/// ```
#[proc_macro_attribute]
pub fn dina_contract(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;

    // Extract existing fields
    let existing_fields = match &input.fields {
        syn::Fields::Named(fields) => {
            let field_iter = fields.named.iter();
            quote! { #(#field_iter,)* }
        }
        _ => {
            return syn::Error::new_spanned(
                &input,
                "dina_contract only supports structs with named fields",
            )
            .to_compile_error()
            .into();
        }
    };

    let expanded = quote! {
        #(#attrs)*
        #[derive(serde::Serialize, serde::Deserialize, borsh::BorshSerialize, borsh::BorshDeserialize)]
        #vis struct #name #generics {
            #existing_fields
            /// Internal storage version marker used by the runtime.
            #[serde(skip, default)]
            #[borsh(skip)]
            pub __storage: u32,
        }

        // WASM entry points — these are called by the Dina WASM runtime.
        #[cfg(target_arch = "wasm32")]
        mod __dina_entry {
            use super::*;

            /// Called once when the contract is first deployed.
            #[no_mangle]
            pub extern "C" fn __init(args_ptr: u32, args_len: u32) -> u64 {
                let args = unsafe {
                    core::slice::from_raw_parts(args_ptr as *const u8, args_len as usize)
                };
                let contract: #name = match borsh::BorshDeserialize::try_from_slice(args) {
                    Ok(c) => c,
                    Err(_) => return 1, // error code
                };
                let serialized = match borsh::BorshSerialize::try_to_vec(&contract) {
                    Ok(s) => s,
                    Err(_) => return 2,
                };
                // Store the contract state via host storage_set at key ""
                unsafe {
                    crate::host::__host_storage_set(
                        b"__state\0".as_ptr() as u32,
                        7,
                        serialized.as_ptr() as u32,
                        serialized.len() as u32,
                    );
                }
                0 // success
            }

            /// Called for every subsequent transaction or query.
            #[no_mangle]
            pub extern "C" fn __call(method_ptr: u32, method_len: u32, args_ptr: u32, args_len: u32) -> u64 {
                let method = unsafe {
                    let bytes = core::slice::from_raw_parts(method_ptr as *const u8, method_len as usize);
                    core::str::from_utf8_unchecked(bytes)
                };
                let args = unsafe {
                    core::slice::from_raw_parts(args_ptr as *const u8, args_len as usize)
                };

                // Load current state
                let state_packed = unsafe {
                    crate::host::__host_storage_get(
                        b"__state\0".as_ptr() as u32,
                        7,
                    )
                };
                let state_ptr = (state_packed >> 32) as u32;
                let state_len = (state_packed & 0xFFFFFFFF) as u32;
                let state_bytes = unsafe {
                    core::slice::from_raw_parts(state_ptr as *const u8, state_len as usize)
                };
                let mut contract: #name = match borsh::BorshDeserialize::try_from_slice(state_bytes) {
                    Ok(c) => c,
                    Err(_) => return 1,
                };

                let result = #name::__dispatch(&mut contract, method, args);

                // Persist state
                let serialized = match borsh::BorshSerialize::try_to_vec(&contract) {
                    Ok(s) => s,
                    Err(_) => return 2,
                };
                unsafe {
                    crate::host::__host_storage_set(
                        b"__state\0".as_ptr() as u32,
                        7,
                        serialized.as_ptr() as u32,
                        serialized.len() as u32,
                    );
                }

                // Write result to memory and return packed ptr|len
                let ptr = result.as_ptr() as u64;
                let len = result.len() as u64;
                (ptr << 32) | len
            }
        }
    };

    expanded.into()
}

/// Attribute macro for impl blocks.
///
/// Scans methods for `#[init]`, `#[payable]`, and `#[view]` attributes,
/// then generates a `__dispatch` function that routes method calls by name.
///
/// # Example
/// ```ignore
/// #[dina_impl]
/// impl MyToken {
///     #[init]
///     pub fn new(name: String, supply: u64) -> Self { ... }
///
///     #[payable]
///     pub fn transfer(&mut self, to: Address, amount: u64) -> TxResult { ... }
///
///     #[view]
///     pub fn balance_of(&self, owner: Address) -> u64 { ... }
/// }
/// ```
#[proc_macro_attribute]
pub fn dina_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);
    let self_ty = &input.self_ty;

    let mut dispatch_arms = Vec::new();
    let mut method_names = Vec::new();
    let mut method_kinds = Vec::new(); // "init", "payable", "view", "mutable"

    for item in &mut input.items {
        if let ImplItem::Fn(method) = item {
            let method_name = method.sig.ident.to_string();
            let mut kind = None;

            // Check for our custom attributes and remove them
            // (they are not real Rust attributes, just markers)
            method.attrs.retain(|attr| {
                if attr.path().is_ident("init") {
                    kind = Some("init");
                    false // remove from output
                } else if attr.path().is_ident("payable") {
                    kind = Some("payable");
                    false
                } else if attr.path().is_ident("view") {
                    kind = Some("view");
                    false
                } else {
                    true // keep other attributes
                }
            });

            let kind = match kind {
                Some(k) => k,
                None => continue, // skip methods without our markers
            };

            // Collect parameter names and types (skip self)
            let mut param_names = Vec::new();
            let mut param_types = Vec::new();
            for arg in method.sig.inputs.iter() {
                if let FnArg::Typed(pat_type) = arg {
                    if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                        param_names.push(pat_ident.ident.clone());
                        param_types.push(pat_type.ty.as_ref().clone());
                    }
                }
            }

            let fn_ident = &method.sig.ident;
            let has_return = !matches!(method.sig.output, ReturnType::Default);

            // Determine if method takes &mut self, &self, or no self
            let takes_mut_self = method
                .sig
                .inputs
                .iter()
                .any(|arg| matches!(arg, FnArg::Receiver(r) if r.mutability.is_some()));
            let takes_self = method
                .sig
                .inputs
                .iter()
                .any(|arg| matches!(arg, FnArg::Receiver(_)));

            // Build the dispatch arm
            let deserialize_params = if param_names.is_empty() {
                quote! {}
            } else {
                // Deserialize a tuple of parameters from the args bytes
                let tuple_type = quote! { (#(#param_types,)*) };
                quote! {
                    let (#(#param_names,)*): #tuple_type =
                        borsh::BorshDeserialize::try_from_slice(args)
                            .expect("failed to deserialize method arguments");
                }
            };

            let call_expr = if takes_mut_self || takes_self {
                quote! { contract.#fn_ident(#(#param_names),*) }
            } else {
                // Static method (like constructors)
                quote! { Self::#fn_ident(#(#param_names),*) }
            };

            let serialize_result = if has_return {
                quote! {
                    let result = #call_expr;
                    borsh::BorshSerialize::try_to_vec(&result)
                        .expect("failed to serialize result")
                }
            } else {
                quote! {
                    #call_expr;
                    Vec::new()
                }
            };

            let arm = quote! {
                #method_name => {
                    #deserialize_params
                    #serialize_result
                }
            };

            dispatch_arms.push(arm);
            method_names.push(method_name.clone());
            method_kinds.push(kind.to_string());
        }
    }

    // Build method registry for introspection
    let registry_entries = method_names
        .iter()
        .zip(method_kinds.iter())
        .map(|(name, kind)| {
            quote! { (#name, #kind) }
        });

    let dispatch_fn = quote! {
        impl #self_ty {
            /// Dispatch a method call by name. Deserializes arguments from `args`,
            /// calls the method, and returns the serialized result.
            pub fn __dispatch(&mut self, method: &str, args: &[u8]) -> Vec<u8> {
                match method {
                    #(#dispatch_arms,)*
                    _ => panic!("unknown method: {}", method),
                }
            }

            /// Returns the contract's method registry for introspection.
            /// Each entry is (method_name, kind) where kind is "init", "payable", or "view".
            pub fn __methods() -> &'static [(&'static str, &'static str)] {
                &[#(#registry_entries),*]
            }
        }
    };

    let expanded = quote! {
        #input
        #dispatch_fn
    };

    expanded.into()
}

/// Marker attribute for constructor methods. Processed by `#[dina_impl]`.
#[proc_macro_attribute]
pub fn init(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marker attribute for methods that can receive USDC. Processed by `#[dina_impl]`.
#[proc_macro_attribute]
pub fn payable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marker attribute for read-only methods. Processed by `#[dina_impl]`.
#[proc_macro_attribute]
pub fn view(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
