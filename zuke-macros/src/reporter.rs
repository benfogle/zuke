use proc_macro::TokenStream;
use quote::quote;

pub fn register_reporter(name: &str, func: syn::ItemFn) -> TokenStream {
    let func_name = func.sig.ident.clone();

    (quote! {
        #func

        const _: () = {
            use ::zuke::reexport::inventory;
            inventory::submit! {
                ::zuke::reporter::ReporterEntry {
                    name: #name.to_string(),
                    func: #func_name,
                }
            }
        };
    })
    .into()
}
