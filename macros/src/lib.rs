#![feature(allocator_api)]
#![feature(proc_macro_quote)]

use std::alloc::Global;
use proc_macro::TokenStream;

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
