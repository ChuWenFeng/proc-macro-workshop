use proc_macro::TokenStream;
use quote::quote;
use syn::parse_quote;

#[proc_macro_derive(CustomDebug,attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as syn::DeriveInput);
    // eprintln!("struct:\n{:#?}",st);
    match do_expand(&st){
        Ok(token_stream)=> token_stream.into(),
        Err(e)=>e.to_compile_error().into(),
    }

}

fn do_expand(st:&syn::DeriveInput)->syn::Result<proc_macro2::TokenStream>{
    let ret =generate_debug_trait(st)?;
    
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
    Err(syn::Error::new_spanned(d, "Must define on a struct,not Enum".to_string()))
}

fn generate_debug_trait(st:&syn::DeriveInput)-> syn::Result<proc_macro2::TokenStream>{
    let fields = get_fields_from_derive_input(st)?;
    let struct_name_ident = &st.ident;

    let struct_name_literal = struct_name_ident.to_string();
    let mut fmt_body_stream = proc_macro2::TokenStream::new();

    fmt_body_stream.extend(quote!(
        debug_struct(#struct_name_literal)
    ));

    for field in fields.iter(){
        let field_name_ident = field.ident.as_ref().unwrap();
        let field_name_literal = field_name_ident.to_string();

        let mut format_str = "{:?}".to_string();
        if let Some(format) = get_custom_format_of_field(field)?{
            format_str = format;
        }

        fmt_body_stream.extend(quote!{
           .field(#field_name_literal,&format_args!(#format_str,self.#field_name_ident))
        });
    }

    fmt_body_stream.extend(quote!(
        .finish()
    ));

    let mut generic_param_to_modify = st.generics.clone();

    let mut field_type_names = Vec::new();
    let mut phantomdata_type_param_names = Vec::new();

    for field in fields{
        if let Some(s) = get_field_type_name(field)?{
            field_type_names.push(s);
        }
        if let Some(s) = get_phantomdata_generic_type_name(field)?{
            phantomdata_type_param_names.push(s);
        }
    }

    for g in generic_param_to_modify.params.iter_mut(){
        if let syn::GenericParam::Type(t) = g{
            let type_param_name = t.ident.to_string();
            if phantomdata_type_param_names.contains(&type_param_name) && !field_type_names.contains(&type_param_name){
                continue;
            }

            t.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }
    let (impl_generics,type_generics,where_clause) = generic_param_to_modify.split_for_impl();

    let ret_stream = quote!(
        impl #impl_generics std::fmt::Debug for #struct_name_ident #type_generics #where_clause{
            fn fmt(&self,fmt:&mut std::fmt::Formatter)-> std::fmt::Result{
                fmt.#fmt_body_stream
            }
        }
    );
    return Ok(ret_stream)
}

fn get_custom_format_of_field(field:&syn::Field)->syn::Result<Option<String>>{
    for attr in &field.attrs{
        // eprintln!("{:#?}",attr.parse_meta()?);
        if let Ok(syn::Meta::NameValue(syn::MetaNameValue{
            ref path,
            ref lit,
            ..
        })) = attr.parse_meta(){
            if path.is_ident("debug"){
                if let syn::Lit::Str(ref ident_str) = lit{
                    return Ok(Some(ident_str.value()));
                }
            }
        }
    }

    Ok(None)
}

fn get_phantomdata_generic_type_name(field: &syn::Field) -> syn::Result<Option<String>> {
    if let syn::Type::Path(syn::TypePath{path: syn::Path{ref segments,..},..}) = field.ty{
        if let Some(syn::PathSegment{ref ident,ref arguments}) = segments.last(){
            if ident == "PhantomData"{
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments{args,..}) = arguments{
                    if let Some(syn::GenericArgument::Type(syn::Type::Path(ref gp))) = args.first(){
                        if let Some(generic_ident) = gp.path.segments.first(){
                            return Ok(Some(generic_ident.ident.to_string()));
                        }
                    }
                }
            }
        }
    }
    return Ok(None);
}

fn get_field_type_name(field:&syn::Field)-> syn::Result<Option<String>>{
    if let syn::Type::Path(syn::TypePath{path: syn::Path{ref segments,..},..}) = field.ty{
        if let Some(syn::PathSegment{ref ident,..}) = segments.last(){
            return Ok(Some(ident.to_string()));
        }
    }
    return Ok(None);
}