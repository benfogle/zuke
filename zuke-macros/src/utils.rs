//! Registers before/after hook functions, and parses tag expressions
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// Adapt a function call to be async -> anyhow::Result<()>
pub fn make_call(
    func_call: TokenStream2,
    func: &syn::ItemFn,
    captures: bool,
    may_cancel: bool,
) -> TokenStream2 {
    // handle asyncness (#1)
    let func_call = match func.sig.asyncness {
        Some(_) => quote! { #func_call.await },
        None => func_call,
    };

    // Handle return type. Assume that any return value is a Result that can be converted to
    // anyhow::Result. (TODO: Handle explicit -> () )
    let func_call = match func.sig.output {
        syn::ReturnType::Default => quote! {
            {
                #func_call;
                ::std::result::Result::<(), ::zuke::reexport::anyhow::Error>::Ok(())
            }
        },
        _ => quote! {
            {
                #func_call?;
                ::std::result::Result::<(), ::zuke::reexport::anyhow::Error>::Ok(())
            }
        },
    };

    // Hande asyncness (#2)
    //
    match func.sig.asyncness {
        Some(_) => {
            // In the async case, we have the option of cancelling
            if may_cancel {
                quote! {
                    let __zuke_flag = context.options().canceled.clone();
                    let fut = async move { #func_call };
                    let canceled = __zuke_flag.wait();
                    use ::zuke::reexport::futures::{pin_mut, future::{Either, select}};
                    pin_mut!(fut);
                    pin_mut!(canceled);
                    match select(fut, canceled).await {
                        Either::Left((result, _)) => result,
                        Either::Right(_) => Err(::zuke::StepError::cancel().into()),
                    }
                }
            } else {
                quote! { #func_call }
            }
        }
        None => {
            // In the blocking case, we assume this will be joined immediately and not sent to a
            // task to live forever. In that case there is nothing (safe) the child thread can do
            // to violate the lifetimes of its arguments, so long as it doesn't just declare them
            // 'static. We will transmute lifetimes so that we can do this on a blocking thread,
            // and immediately await it.  We can't cancel here, and dropping the future awaiting
            // the thread may cause a crash. (That's handled on our end: the user shouldn't have to
            // worry about it.) We also need to do our own panic handling.
            let captures = if captures {
                quote! {
                    let captures = unsafe {
                        ::std::mem::transmute::<
                            &::zuke::reexport::regex::Captures<'_>,
                            &'static ::zuke::reexport::regex::Captures<'static>>(captures)
                    };
                }
            } else {
                quote! {}
            };

            quote! {
                {
                    let context = unsafe {
                        ::std::mem::transmute::<
                            &mut ::zuke::Context,
                            &'static mut ::zuke::Context>(context)
                    };

                    #captures

                    ::async_std::task::spawn_blocking(move || {
                        ::zuke::PanicToError::from(|| #func_call).call_once()
                    }).await
                }
            }
        }
    }
}
