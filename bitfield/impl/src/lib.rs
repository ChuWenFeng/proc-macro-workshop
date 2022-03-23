use std::{collections::HashSet};

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
    let mut struct_visit = StructVisitor::new();
    
    struct_visit.visit_item_struct_mut(st);

    if let Some(e) = struct_visit.err{
        return syn::Result::Err(e);
    }
    let mut fields_ty = HashSet::new();
    for (_,ty_size,_) in struct_visit.fields_seq.iter(){
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
    let st_vis = st.vis.clone();
    let st_token = st.struct_token.clone();
    let st_ident = st.ident.clone();
    let st_size = struct_visit.size / 8;

    token_stream.extend(quote::quote! {
        #st_vis #st_token #st_ident {
            data: [u8;#st_size],
        }
    });

    let fn_impl_token =  impl_st_fn_geter_seter(&struct_visit)?;
    token_stream.extend(quote::quote!{
        impl #st_ident{

            pub fn new()->Self{
                #st_ident{data:[0;#st_size]}
            }

            #fn_impl_token
        }
    });

    // eprintln!("token_stream:\n{:#?}",token_stream);
    
    
    Ok(token_stream)
}

fn impl_st_fn_geter_seter(st:&StructVisitor)->syn::Result<proc_macro2::TokenStream>{
    let mut ret = proc_macro2::TokenStream::new();

    ret.extend(quote::quote! {
        fn seter_to_idx(&mut self,idx_off:usize,size:usize,input:u64){
            // println!("seter");
            let mut off_byte:usize = idx_off / 8;
            let mut off_bit:usize = idx_off % 8;
            let flag = off_bit == 0;
            let mut size = size;
            let mut data_off:usize = 0;

            let mut set_byte:u8 = self.data[off_byte];
            if !flag{
                set_byte = set_byte & ((1 << off_bit) -1);
                while size > 0 && off_bit !=0 && off_bit != 8{
                    let tmp:u8 = ((input as u8) & (1 << data_off)) << (off_bit-data_off);
                    set_byte += tmp;
                    off_bit +=1;
                    data_off+=1;
                    size -= 1;
                }
                self.data[off_byte] = set_byte;
                
                off_byte+=1;
            }
            
            while size / 8 >0{
                set_byte = (input >> data_off) as u8;
                self.data[off_byte] = set_byte;
                data_off +=8;
                off_byte+=1;
                size -=8;
            }
            if size == 0{
                return;
            }

            set_byte = self.data[off_byte];
            set_byte = set_byte - (set_byte & ( (1 << (data_off % 8)) - 1 ));
            while size<8 && size>0{
                set_byte += (input >> data_off) as u8 & 1;

                data_off +=1;
                size -=1;
            }
            
        }
        fn geter_to_idx(&self,idx_off:usize,size:usize)->u64{
            // println!("geter");
            let mut off_byte:usize = idx_off / 8;
            let mut off_bit:usize = idx_off % 8;
            let flag = off_bit == 0;
            let mut size = size;
            let mut ret:u64 = 0;
            let mut data_off = 0;

            if !flag{
                while size > 0 && off_bit !=0 && off_bit != 8{
                    let data_sli = self.data[off_byte];
                    ret += (((data_sli & (1 << off_bit))) >> (off_bit - data_off))as u64;

                    off_bit += 1;
                    data_off+=1;
                    size-=1;
                }
                off_byte += 1;
            }

            while size / 8 > 0{
                let data_sli = self.data[off_byte];
                ret += (data_sli as u64) << data_off;

                off_byte += 1;
                data_off+=8;
                size -= 8;
            }

            while size % 8 >0{
                let data_sli = self.data[off_byte];
                ret += ((data_sli & (1 <<data_off % 8)) as u64) << data_off;

                data_off += 1;
                size -=1;
            }

            ret
        }

    });

    let mut idx_off = 0;
    for (name,size,ty) in st.fields_seq.iter(){
        let seter_name = format!("set_{}",name);
        let seter_ident = syn::Ident::new(&seter_name, proc_macro2::Span::call_site());
        let geter_name = format!("get_{}",name);
        let geter_ident = syn::Ident::new(&geter_name, proc_macro2::Span::call_site());
        let ty_ident = syn::Ident::new(ty, proc_macro2::Span::call_site());
        ret.extend(quote::quote! {
            pub fn #seter_ident(&mut self,input:#ty_ident){
                // println!("seter");
                self.seter_to_idx(#idx_off,#size as usize,input as u64);
                
            }
            pub fn #geter_ident(&self)->#ty_ident{
                // println!("geter");
                self.geter_to_idx(#idx_off,#size as usize) as #ty_ident
            }

        });

        idx_off += *size as usize;
    }


    Ok(ret)
}

fn get_function_param_ty(size:u8)->Result<String,String>{
    match size{
        1..=8 => Ok("u8".to_string()),
        9..=16 => Ok("u16".to_string()),
        17..=32 =>Ok("u32".to_string()),
        33..=64 =>Ok("u64".to_string()),
        _ => Err("field size must between 1 to 64 ".to_string()),
    }
}

struct StructVisitor{
    fields_seq:Vec<(String,u8,String)>,
    size:usize,
    err:Option<syn::Error>,
}

impl StructVisitor{
    fn new()->Self{
        StructVisitor { fields_seq: vec![], size:0, err:None }
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
            let field_ty;
            match get_function_param_ty(ty_size){
                Ok(s) => field_ty = s,
                Err(s) =>{
                    self.err = Some(syn::Error::new_spanned(field, s));
                    return;
                }
            }
            self.fields_seq.push((name,ty_size,field_ty));
            str_size += ty_size as usize;
        }

        if str_size %8 != 0{
            self.err = Some(syn::Error::new_spanned(i, "fields size must be multiple 8"));
            return;
        }
        self.size = str_size;
        syn::visit_mut::visit_fields_named_mut(self, i);
    }
}
