
use proc_macro::TokenStream;
use syn::{self, spanned::Spanned};
use quote::{ quote};

#[proc_macro_derive(Builder,attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as syn::DeriveInput);
    match do_expand(&st) {
        Ok(token_stream) => token_stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn do_expand(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // eprintln!("{:#?}",st.data);
    let struct_name_literal = st.ident.to_string();
    let builder_name_literal = format!("{}Builder", struct_name_literal);
    let builder_name_ident = syn::Ident::new(&builder_name_literal, st.span());

    let struct_ident = &st.ident;  // 模板代码中不可以使用`.`来访问结构体成员，所以要在模板代码外面将标识符放到一个独立的变量中

    let fields = get_fields_from_derive_input(st)?;
    // eprintln!("{:#?}",fields);
    let builder_struct_fields_def = generate_builder_struct_fields_def(fields)?;
    let builder_struct_factory_init_clauses = generate_builder_struct_factory_init_clauses(fields)?;

    let setter_functions = generate_setter_functions(fields)?;
    let generated_builder_functions = generate_build_function(fields,struct_ident)?;

    let ret = quote! {     
        pub struct #builder_name_ident {                  
            #builder_struct_fields_def                                     
        }                                                 
        impl #struct_ident {                              
            pub fn builder() -> #builder_name_ident {
                #builder_name_ident{
                    #(#builder_struct_factory_init_clauses),*
                }
            } 
        }      
        impl #builder_name_ident{
            #setter_functions

            #generated_builder_functions                                                                     
        }                                            
    };                    

    return Ok(ret);
}
type StructFields = syn::punctuated::Punctuated<syn::Field,syn::Token!(,)>;


fn get_fields_from_derive_input(d:&syn::DeriveInput)->syn::Result<&StructFields>{

    if let syn::Data::Struct(syn::DataStruct{
        fields: syn::Fields::Named(syn::FieldsNamed{ref named,..}),
        ..
    }) = d.data{
        return Ok(named)
    }
    
    Err(syn::Error::new_spanned(d,"Must define on a Struct,not Enum".to_string()))
}

fn generate_builder_struct_fields_def(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream>{
    let idents:Vec<_> = fields.iter().map(|f| {&f.ident}).collect();
    let types:Vec<_> = fields.iter().map(|f|{
        if let Some(inner_ty) = get_generic_inner_type(&f.ty,"Option") {
            quote!(std::option::Option<#inner_ty>)
        } else if get_user_specified_ident_for_vec(f).is_some(){
            let origin_ty = &f.ty;
            quote!(#origin_ty)
        }else{
            let origin_ty = &f.ty;
            quote!(std::option::Option<#origin_ty>)
        }
    }).collect();
    let token_stream = quote!{
        #(#idents: #types),*
    };

    Ok(token_stream)
}

fn generate_builder_struct_factory_init_clauses(fields: &StructFields) -> syn::Result<Vec<proc_macro2::TokenStream>>{
    let init_clauses:Vec<_> = fields.iter().map(|f|{
        let ident = &f.ident;
        if get_user_specified_ident_for_vec(f).is_some(){
            quote!{
                #ident: std::vec::Vec::new()
            }
        }else{
            quote!{
                #ident: std::option::Option::None
            }
        }
    }).collect();
    Ok(init_clauses)
}

fn generate_setter_functions(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream>{
    let idents_types:Vec<_> = fields.iter().map(|f|(&f.ident,&f.ty)).collect();
    let mut final_tokenstream = proc_macro2::TokenStream::new();

    for (idx,(ident,type_)) in idents_types.iter().enumerate(){
        let mut token_s;
        if let Some(inner_ty) = get_generic_inner_type(type_,"Option"){
            token_s = quote!{
                fn #ident(&mut self,input: #inner_ty)->&mut Self{ 
                    self.#ident = std::option::Option::Some(input);
                    self
                }
            }
        }else if let Some(ref user_specified_ident) = get_user_specified_ident_for_vec(&fields[idx]){
            let inner_ty = get_generic_inner_type(type_,"Vec").ok_or(syn::Error::new(fields[idx].span(),"each field must be specified with Vec field"))?;
            token_s = quote! {
                fn #user_specified_ident(&mut self,input:#inner_ty)-> &mut Self{
                    self.#ident.push(input);
                    self
                }
            };
            if user_specified_ident != ident.as_ref().unwrap(){
                token_s.extend(
                    quote!{
                        fn #ident(&mut self,input: #type_)->&mut Self{
                            self.#ident = input.clone();
                            self
                        }
                    }
                );
            }
        }else{
            token_s = quote! {
                pub fn #ident (&mut self,input: #type_) -> &mut Self{
                    self.#ident = std::option::Option::Some(input);
                    self
                }
            };
        }
        
        final_tokenstream.extend(token_s);
    }   
    Ok(final_tokenstream)
}

fn generate_build_function(fields: &StructFields, origin_struct_ident: &syn::Ident) -> syn::Result<proc_macro2::TokenStream>{

    let idents:Vec<_> = fields.iter().map(|f|&f.ident).collect();
    let types:Vec<_> = fields.iter().map(|f|&f.ty).collect();
    let mut checker_code_pieces = Vec::new();

    for idx in 0..idents.len(){
        let ident = idents[idx];
        let type_ = types[idx];
        if get_generic_inner_type(type_,"Option").is_none() && get_user_specified_ident_for_vec(&fields[idx]).is_none(){
            checker_code_pieces.push(quote!{
                if self.#ident.is_none(){
                    let err = format!("{} field missing",stringify!(#ident));
                    return std::result::Result::Err(err.into())
                }
            });
        }
        
    }

    let mut fill_result_clauses = Vec::new();
    for idx in 0..idents.len(){
        let ident = idents[idx];
        if get_user_specified_ident_for_vec(&fields[idx]).is_some(){
            fill_result_clauses.push(quote!{
                #ident:self.#ident.clone()
            });
        }else if get_generic_inner_type(types[idx],"Option").is_none(){
            fill_result_clauses.push(quote!{
                #ident: self.#ident.clone().unwrap()
            });
        }else{
            fill_result_clauses.push(quote!{
                #ident:self.#ident.clone()
            });
        }
        
    }

    let token_stream = quote! {
        pub fn build(&mut self)-> std::result::Result<#origin_struct_ident,std::boxed::Box<dyn std::error::Error>>{
            #(#checker_code_pieces)*

            let ret = #origin_struct_ident{
                #(#fill_result_clauses),*
            };
            std::result::Result::Ok(ret)
        }
    };

    Ok(token_stream)

}

fn get_generic_inner_type<'a>(ty:&'a syn::Type,outer_ident_name:&str)->Option<&'a syn::Type>{
    if let syn::Type::Path(syn::TypePath{ref path,..}) = ty{
        if let Some(seg) = path.segments.last(){
            if seg.ident == outer_ident_name{
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments{
                    ref args,
                    ..
                }) = seg.arguments{
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.first(){
                        return Some(inner_ty);
                    }
                }
            }
        }
    }                       
    None
}

fn get_user_specified_ident_for_vec(field:&syn::Field)-> Option<syn::Ident>{
    for attr in &field.attrs{
        if let Ok(syn::Meta::List(syn::MetaList{
            ref path,
            ref nested,
            ..
        })) = attr.parse_meta(){
            if let Some(p) = path.segments.first(){
                if p.ident == "builder"{
                    if let Some(syn::NestedMeta::Meta(syn::Meta::NameValue(kv))) = nested.first(){
                        if kv.path.is_ident("each"){
                            if let syn::Lit::Str(ref ident_str) = kv.lit{
                                return Some(syn::Ident::new(
                                    ident_str.value().as_str(),
                                    attr.span(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}