use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{visit_mut::VisitMut, token::Underscore};

#[proc_macro_attribute]
pub fn sorted(_args: TokenStream, input: TokenStream) -> TokenStream {
    // eprintln!("sorted");
    let st = syn::parse_macro_input!(input as syn::Item);

    match do_expand(&st) {
        Ok(token_stream) =>  token_stream.into(),
        Err(e) =>  {
            let mut t = st.to_token_stream();
            t.extend(e.to_compile_error());
            t.into()
        },
    }
}

fn do_expand(st: &syn::Item)-> syn::Result<proc_macro2::TokenStream>{

    let ret = match st{
        syn::Item::Enum(enum_node)=> check_enum_order(enum_node),
        _ => syn::Result::Err(syn::Error::new(proc_macro2::Span::call_site(), "expected enum or match expression")),
    }?;
    return Ok(ret);
}

fn check_enum_order(st: &syn::ItemEnum) -> syn::Result<proc_macro2::TokenStream>{
    let origin_names:Vec<_> = st.variants.iter().map(|item|{(item.ident.to_string(),item)}).collect();

    let mut sorted_origin_names = origin_names.clone();
    sorted_origin_names.sort_by(|a,b|{a.0.cmp(&b.0)});

    for (a,b) in origin_names.iter().zip(sorted_origin_names.iter()){
        if a.0 != b.0{
            return syn::Result::Err(syn::Error::new(b.1.ident.span(), format!("{} should sort before {}", b.0, a.0)));
        }
    }
    return syn::Result::Ok(st.to_token_stream());

}
#[proc_macro_attribute]
pub fn check(_args: TokenStream,input: TokenStream) -> TokenStream{
    // eprintln!("check");
    let mut st = syn::parse_macro_input!(input as syn::ItemFn);

    match do_match_expand(&mut st){
        Ok(token_stream) => token_stream.into(),
        Err(e) => {
            let mut t = e.to_compile_error();
            t.extend(st.to_token_stream());
            t.into()
        }
    
    }
}

fn do_match_expand(st:&mut syn::ItemFn) -> syn::Result<proc_macro2::TokenStream>{
    // eprintln!("syn::ItemFn:\n{:#?}",st);
    let mut visitor = MatchVisitor{err:None};

    visitor.visit_item_fn_mut(st);

    if visitor.err.is_none(){
        return syn::Result::Ok(st.to_token_stream())
    }else{
        return syn::Result::Err(visitor.err.unwrap())
    }

}

fn get_path_string(p: &syn::Path) -> String{
    // eprintln!("syn::Path:\n{:#?}",p);
    let mut buf = Vec::new();
    for i in &p.segments{
        buf.push(i.ident.to_string());
    }
    return buf.join("::")
}

struct MatchVisitor{
    err:Option<syn::Error>,
}

impl syn::visit_mut::VisitMut for MatchVisitor{
    fn visit_expr_match_mut(&mut self,i: &mut syn::ExprMatch){
        let mut target_idx:isize = -1;
        for (idx,attr) in i.attrs.iter().enumerate(){
            if get_path_string(&attr.path) == "sorted"{
                target_idx = idx as isize;
                break;
            }
        }

        if target_idx != -1{
            i.attrs.remove(target_idx as usize);
            let mut match_arm_names:Vec<(_,&dyn ToTokens)> = Vec::new();
            for arm in &i.arms{
                match &arm.pat{
                    syn::Pat::Path(p)=> {
                        // eprintln!("syn::Pat::Path");
                        match_arm_names.push((get_path_string(&p.path),&p.path));
                    },
                    syn::Pat::TupleStruct(p) =>{
                        // eprintln!("syn::Pat::TupleStruct");
                        match_arm_names.push((get_path_string(&p.path),&p.path));
                    },
                    syn::Pat::Struct(p)=>{
                        // eprintln!("syn::Pat::Struct");
                        match_arm_names.push((get_path_string(&p.path),&p.path));
                    },
                    syn::Pat::Ident(p)=>{
                        match_arm_names.push((p.ident.to_string(),&p.ident));
                    },
                    syn::Pat::Wild(p)=>{
                        match_arm_names.push(("_".to_string(),&p.underscore_token));
                    },
                    _ => {
                        self.err = Some(syn::Error::new_spanned(&arm.pat, "unsupported by #[sorted]"));
                        return;
                    }

                }
            }
            

            let mut sorted_names = match_arm_names.clone();
            sorted_names.sort_by(|a,b|{a.0.cmp(&b.0)});
            for (idx,(a,b)) in match_arm_names.iter().zip(sorted_names.iter()).enumerate() {
                if a.0 == "_".to_string() && idx != match_arm_names.len(){
                    self.err = Some(syn::Error::new_spanned(a.1, "\"_\" should be last one"));
                    return;
                }
                if a.0 != b.0{
                    self.err = Some(syn::Error::new_spanned(b.1, format!("{} should sort before {}", b.0, a.0)));
                    return;
                }
                
            }
   
        }

        syn::visit_mut::visit_expr_match_mut(self, i)

    }
}