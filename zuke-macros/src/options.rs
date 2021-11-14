use proc_macro::TokenStream;
use quote::quote;

pub fn register_options(func: syn::ItemFn) -> TokenStream {
    let func_name = func.sig.ident.clone();

    (quote! {
        #func

        const _: () = {
            use ::zuke::reexport::inventory;
            inventory::submit! {
                ::zuke::options::ExtraOptionsFunc::from(#func_name)
            }
        };
    })
    .into()
}
