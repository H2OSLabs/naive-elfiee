use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, punctuated::Punctuated, Expr, ExprLit, ItemFn, Lit, Meta, Token};

/// Procedural macro to define capabilities with minimal boilerplate.
///
/// Usage:
/// ```
/// #[capability(id = "core.link", target = "core/*")]
/// fn handle_link(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
///     // implementation
/// }
/// ```
///
/// This generates a struct implementing CapabilityHandler trait.
/// The handler function must accept Option<&Block> as the second parameter.
#[proc_macro_attribute]
pub fn capability(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, Token![,]>::parse_terminated);
    let input_fn = parse_macro_input!(input as ItemFn);

    // Extract capability ID and target from attributes
    let mut cap_id: Option<String> = None;
    let mut target: Option<String> = None;

    for meta in args {
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("id") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = nv.value
                {
                    cap_id = Some(lit_str.value());
                }
            } else if nv.path.is_ident("target") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = nv.value
                {
                    target = Some(lit_str.value());
                }
            }
        }
    }

    let cap_id = cap_id.expect("capability macro requires 'id' attribute");
    let target = target.expect("capability macro requires 'target' attribute");

    // Derive struct name from capability ID
    // "core.link" -> "CoreLinkCapability"
    let struct_name = derive_struct_name(&cap_id);

    // Get the handler function name
    let fn_name = &input_fn.sig.ident;

    // Generate the code
    let expanded = quote! {
        // Generate the capability struct
        pub struct #struct_name;

        // Implement CapabilityHandler trait
        impl crate::capabilities::core::CapabilityHandler for #struct_name {
            fn cap_id(&self) -> &str {
                #cap_id
            }

            fn target(&self) -> &str {
                #target
            }

            fn handler(
                &self,
                cmd: &crate::models::Command,
                block: Option<&crate::models::Block>,
            ) -> crate::capabilities::core::CapResult<Vec<crate::models::Event>> {
                #fn_name(cmd, block)
            }
        }

        // Keep the original function
        #input_fn
    };

    TokenStream::from(expanded)
}

/// Derive a struct name from a capability ID.
///
/// Examples:
/// - "core.create" -> "CoreCreateCapability"
/// - "markdown.write" -> "MarkdownWriteCapability"
/// - "diagram.render" -> "DiagramRenderCapability"
fn derive_struct_name(cap_id: &str) -> proc_macro2::Ident {
    let parts: Vec<&str> = cap_id.split('.').collect();

    let name = parts
        .iter()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first_char) => {
                    let mut capitalized = first_char.to_uppercase().to_string();
                    capitalized.push_str(chars.as_str());
                    capitalized
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("");

    format_ident!("{}Capability", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_struct_name() {
        assert_eq!(
            derive_struct_name("core.create").to_string(),
            "CoreCreateCapability"
        );
        assert_eq!(
            derive_struct_name("core.link").to_string(),
            "CoreLinkCapability"
        );
        assert_eq!(
            derive_struct_name("markdown.write").to_string(),
            "MarkdownWriteCapability"
        );
        assert_eq!(
            derive_struct_name("diagram.render").to_string(),
            "DiagramRenderCapability"
        );
    }
}
