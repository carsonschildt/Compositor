use std::fmt::{Debug};

use quote::ToTokens;
use syn::{punctuated::Punctuated, token::{Brace}, Token, Ident, parse::{Parse, ParseStream}, braced};

use crate::avvalue::AvValue;

pub struct ConfigDelcaration {
    pub ident : Ident,
    _brace_token: Brace,
    pub fields : Punctuated<AvMacro, Token![,]>
}

impl Parse for ConfigDelcaration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(ConfigDelcaration {
            ident : input.parse()?,
            _brace_token: braced!(content in input),
            fields: content.parse_terminated(AvMacro::parse)?,
        })
    }
}

pub struct AvMacro {
    // Comment for users
    description: syn::LitStr,

    // Macro declaration itself with its parameters
    avmacro : (String, Vec<String>),

    // Separates the macro
    _delim : Token![=>],

    default : AvValue,
}

impl AvMacro {
    pub fn description(&self) -> String {
        self.description.value()
    }

    pub fn default(&self) -> &AvValue {
        &self.default
    }

    pub fn av_macro(&self) -> (String, Vec<String>) {
        self.avmacro.clone()
    }
}

impl Parse for AvMacro {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(AvMacro {
            description: input.parse()?,
            avmacro : {
                match input.peek2(Token![=>]) {
                    true => {
                        // Macro has no params.
                        let ident : syn::Ident = input.parse()?;
                        (ident.to_string(), vec![])
                    },
                    false => {
                        let exp : syn::ExprCall = input.parse()?;
                        (
                            exp.func.to_token_stream().to_string(),
                            exp.args.into_iter().map(|t| t.to_token_stream().to_string()).collect()
                        )
                    }
                }
            },
            _delim: input.parse()?,
            default : input.parse()?
        })
    }
}

impl Debug for AvMacro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "AvMacro Declaration:\n    Name: {}\n    Parameters:\n",
           &self.avmacro.0,
        )?;

        for param in (&self.avmacro.1).into_iter() {
            let n = param.to_token_stream().to_string();
            write!(f, "      {n}\n")?;
        }

        write!(f, "\n")
    }
}




impl Debug for ConfigDelcaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConfigDeclaration:\n")?;

        for field in (&self.fields).into_iter() {
            write!(f, "  {:?}", field)?;
        }

        Ok(())
    }
}

