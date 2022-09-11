#![feature(allocator_api)]
#![feature(proc_macro_quote)]

use std::alloc::Global;
use proc_macro::TokenStream;
use quote::quote;

use poise::Command;

#[proc_macro_attribute]
pub fn add_command(_attr: TokenStream, item: TokenStream) -> TokenStream {;
    let item_tokens = proc_macro2::TokenStream::from(item);
    let attr_tokens = proc_macro2::TokenStream::from(item);
    
    let parse = syn::parse2::<Command<_, _>>(item_tokens).expect("Failed to parse tokens.");
    
    let mut command_list = syn::parse2::<Vec<Command<_, _> Global>>(attr_tokens).expect("Failed to parse tokens.")

    match parse {
        syn::Item::Fn(func) => command_list.append(func),
        _ => panic!("Only functions are currently supported!"),
    }
    item
}

#[proc_macro_attribute]
pub fn cmd(attr: TokenStream, item: TokenStream) -> TokenStream {
    assert!(
        attr.is_empty(),
        "Macro cmd must be used as a bare attribute without any arguments or parameters."
    );

    let mut fn_item = syn::parse_macro_input!(item as syn::ItemFn)
    let vis = &fn_item.vis;
    let cmd_name = &fn_item.sig.ident;
    let (fn_args, fn_arg_idents): (Vec<_>, Vec<_>) = fn_item
        .sig
        .inputs
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, mut fn_args)| {
            let pat_type = match &mut fn_arg {
                syn::FnArg::Receiver(_) => unreachable!(),
                syn::FnArg::Typed(it) => it,
            };
            let ident = qoute::format_ident!("arg_{}", i);
            *pat_type.pat = syn::parse_quote(#ident);
            (fn_arg, ident)
        })
        .unzip();

    let attrs = std::mem::take(&mut fn_item.attrs);

    let result = quote! {
        #[::serenity::framework::standard::macros::command]
        #(#attrs)*
        #vis async fn #cmd(#(#fn_args),*) -> ::serenity::framework::standard::CommandResult {
            if let Err(err) = #cmd(#(#fn_arg_idents),*).await {
                arg_1.channel_id
                    .send_message(arg_0, |it| err.create_msg(it))
                    .await
                    .unwrap();
            }

            return Ok(());

            #fn_item
        }
    };

    result.into()
}
