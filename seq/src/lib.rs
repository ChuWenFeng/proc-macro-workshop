use proc_macro::TokenStream;
use syn::Token;
struct SeqParser{
    variable_ident: syn::Ident,
    start:i64,
    end:i64,
    body:proc_macro2::TokenStream,
}

impl syn::parse::Parse for SeqParser{
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self>{
        let variable_ident: syn::Ident = input.parse()?;

        input.parse::<syn::Token!(in)>()?;

        let start: syn::LitInt = input.parse()?;

        input.parse::<Token!(..)>()?;

        let end:syn::LitInt = input.parse()?;

        let body_buf;
        syn::braced!(body_buf in input);

        let body: proc_macro2::TokenStream = body_buf.parse()?;

        let t = SeqParser{
            variable_ident,
            start:start.base10_parse()?,
            end:end.base10_parse()?,
            body,
        };

        Ok(t)
    }

}

impl SeqParser{
    fn expand(&self,ts:&proc_macro2::TokenStream,n:i64)-> proc_macro2::TokenStream{
        let buf = ts.clone().into_iter().collect::<Vec<_>>();

        let mut ret = proc_macro2::TokenStream::new();

        let mut idx = 0;
        while idx < buf.len(){
            let tree_node = &buf[idx];
            match tree_node{
                proc_macro2::TokenTree::Group(g)=>{
                    let new_stream = self.expand(&g.stream(), n);
                    let wrap_in_group = proc_macro2::Group::new(g.delimiter(),new_stream);
                    ret.extend(quote::quote! (#wrap_in_group));
                }
                proc_macro2::TokenTree::Ident(prefix)=>{
                    if idx + 2 < buf.len(){
                        if let proc_macro2::TokenTree::Punct(p) = &buf[idx+1]{
                            if p.as_char() == '~'{
                                if let proc_macro2::TokenTree::Ident(i) = &buf[idx+2]{
                                    if i == &self.variable_ident 
                                        && prefix.span().end() == p.span().start() 
                                        && p.span().end() == i.span().start()
                                        {
                                            let new_ident_litral = format!("{}{}",prefix.to_string(),n);
                                            let new_ident = proc_macro2::Ident::new(&new_ident_litral.as_str(),prefix.span());
                                            ret.extend(quote::quote! {#new_ident});
                                            idx+=3;
                                            continue;
                                        }
                                }
                            }
                        }
                    }

                    if prefix == &self.variable_ident{
                        let new_ident = proc_macro2::Literal::u64_unsuffixed(n as u64);
                        ret.extend(quote::quote! {#new_ident});
                        idx +=1;
                        continue;
                    }
                    ret.extend(quote::quote! {#tree_node});
                }
                _ =>{
                    ret.extend(quote::quote! (#tree_node));
                }
            }

            idx+=1;
        }
        ret
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as SeqParser);
    // eprintln!("st.variable_ient:{}",st.variable_ident.to_string());
    // eprintln!("st.body:{}",st.body);

    let mut ret = proc_macro2::TokenStream::new();
    for i in st.start..st.end{
        ret.extend(st.expand(&st.body, i));
    }


    // eprintln!("TokenStream:\n{}",ret);
    return ret.into();
    // proc_macro::TokenStream::new()
}
