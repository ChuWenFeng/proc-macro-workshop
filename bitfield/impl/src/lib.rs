use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{visit_mut::VisitMut};

#[proc_macro_attribute]
pub fn bitfield(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut st = syn::parse_macro_input!(input as syn::Item);
    // eprintln!("{:#?}",st);

    match do_expand(&mut st){
        Ok(token_stream) => token_stream.into(),
        Err(e) => {
            let mut t = e.to_compile_error();
            t.extend(st.to_token_stream());
            t.into()
        },
    }
}

fn do_expand(st:&mut syn::Item) -> syn::Result<proc_macro2::TokenStream>{

    let ret = match st{
        syn::Item::Struct(struct_ndoe) =>{
            impl_user_define_bit_width(struct_ndoe)
        },
        _ => syn::Result::Err(syn::Error::new(proc_macro2::Span::call_site(), "expected enum or match expression")),
    }?;
    
    Ok(ret)
}

fn impl_user_define_bit_width(st: &mut syn::ItemStruct)->syn::Result<proc_macro2::TokenStream>{
    let mut token_stream = proc_macro2::TokenStream::new();
    // token_stream.extend(st.to_token_stream());
    token_stream.extend(quote::quote! {
        trait Specifier {
            const BITS:u8;
        }
    });
    // let struct_name = st.ident.to_string();
    let mut struct_visit = StructVisitor::new();
    
    struct_visit.visit_item_struct_mut(st);

    if let Some(e) = struct_visit.err{
        return syn::Result::Err(e);
    }
    let mut fields_ty = HashSet::new();
    for (_,ty_size) in struct_visit.fields_seq{
        fields_ty.insert(ty_size);
    }

    for ty_size in fields_ty{
        let struct_name = format!("B{}",ty_size);
        let struct_ident = syn::Ident::new(&struct_name, proc_macro2::Span::call_site());
        token_stream.extend(quote::quote! {
            struct #struct_ident {}
            impl Specifier for #struct_ident{
                const BITS:u8 = #ty_size;
            }
        })
    }

    // eprintln!("token_stream:\n{:#?}",token_stream);
    
    
    Ok(token_stream)
}

struct StructVisitor{
    fields_seq:Vec<(String,u8)>,
    err:Option<syn::Error>,
}

impl StructVisitor{
    fn new()->Self{
        StructVisitor { fields_seq: vec![],err:None }
    }
}

impl syn::visit_mut::VisitMut for StructVisitor{
    fn visit_fields_named_mut(&mut self, i: &mut syn::FieldsNamed) {
        let mut str_size = 0;
        for field in i.named.iter(){
            let name;
            let mut ty = "".to_string();
            if let Some(i) = field.ident.as_ref(){
                name = i.to_string();
            }else{
                self.err = Some(syn::Error::new_spanned(field, "parse field ident err"));
                return;
            }
            if let syn::Type::Path(p) = &field.ty{
                if let Some(ps) = p.path.segments.first(){
                    ty = ps.ident.to_string();
                    if let Some('B') = ty.chars().next(){
                    }else{
                        continue;
                    }
                }else{
                    self.err = Some(syn::Error::new_spanned(field, "parse field ty err"));
                    return;
                }
            }
            let ty_size:u8 = ty.chars().skip(1).collect::<String>().parse().unwrap();
            self.fields_seq.push((name,ty_size));
            str_size += ty_size as u32;
        }

        if str_size %8 != 0{
            self.err = Some(syn::Error::new_spanned(i, "fields size must be mutltiple 8"));
            return;
        }

        syn::visit_mut::visit_fields_named_mut(self, i);
    }
}
