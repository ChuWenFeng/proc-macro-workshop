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
        
        let mut inc = false;
        if input.peek(syn::Token!(=)){
            input.parse::<syn::Token!(=)>()?;
            inc = true;
        }

        let end:syn::LitInt = input.parse()?;

        let body_buf;
        syn::braced!(body_buf in input);

        let body: proc_macro2::TokenStream = body_buf.parse()?;

        let mut t = SeqParser{
            variable_ident,
            start:start.base10_parse()?,
            end:end.base10_parse()?,
            body,
        };

        if inc{
            t.end +=1;
        }

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

    fn find_block_to_expand_and_to_expand(&self,c:syn::buffer::Cursor)->(proc_macro2::TokenStream,bool){
        let mut found = false;
        let mut ret = proc_macro2::TokenStream::new();

        let mut cursor = c;

        while !cursor.eof(){
            if let Some((punct_prefix,cursor_1)) = cursor.punct(){
                if punct_prefix.as_char() == '#'{
                    if let Some((group_cur,_,cursor_2)) = cursor_1.group(proc_macro2::Delimiter::Parenthesis){
                        if let Some((punct_suffix,cursor_3)) = cursor_2.punct(){
                            if punct_suffix.as_char() == '*'{
                                for i in self.start..self.end{
                                    let t = self.expand(&group_cur.token_stream(), i);
                                    ret.extend(t);
                                }
                                cursor = cursor_3;
                                found = true;
                                continue;
                            }
                        }
                    }
                }
            }

            if let Some((group_cur,_,next_cur)) = cursor.group(proc_macro2::Delimiter::Brace){
                let (t,f) = self.find_block_to_expand_and_to_expand(group_cur);
                found = f;
                ret.extend(quote::quote!( {#t} ));
                cursor = next_cur;
                continue;
            }else if let Some((group_cur,_,next_cur)) = cursor.group(proc_macro2::Delimiter::Bracket){
                let (t,f) = self.find_block_to_expand_and_to_expand(group_cur);
                found = f;
                ret.extend(quote::quote!( [#t] ));
                cursor = next_cur;
                continue;
            }else if let Some((group_cur,_,next_cur)) = cursor.group(proc_macro2::Delimiter::Parenthesis) {
                let (t,f) = self.find_block_to_expand_and_to_expand(group_cur);
                found = f;
                ret.extend(quote::quote!( (#t) ));
                cursor = next_cur;
                continue;
            }else if let Some((punct,next_cur)) = cursor.punct() {
                ret.extend(quote::quote! (#punct));
                cursor = next_cur;
                continue;
            }else if let Some((ident,next_cur)) = cursor.ident(){
                ret.extend(quote::quote!(#ident));
                cursor = next_cur;
                continue;
            } else if let Some((literal,next_cur)) = cursor.literal() {
                ret.extend(quote::quote! (#literal));
                cursor = next_cur;
                continue;
            }else if let Some((lifetime,next_cur)) = cursor.lifetime(){
                ret.extend(quote::quote! (#lifetime));
                cursor = next_cur;
                continue;
            }
        }
        return (ret,found);
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as SeqParser);
    // eprintln!("st.variable_ient:{}",st.variable_ident.to_string());
    // eprintln!("st.body:{}",st.body);

    let mut ret = proc_macro2::TokenStream::new();

    let buffer = syn::buffer::TokenBuffer::new2(st.body.clone());

    let (ret_1,expand) = st.find_block_to_expand_and_to_expand(buffer.begin());
    if expand{
        return ret_1.into();
    }

    for i in st.start..st.end{
        ret.extend(st.expand(&st.body, i));
    }


    // eprintln!("TokenStream:\n{}",ret);
    return ret.into();
    // proc_macro::TokenStream::new()
}
