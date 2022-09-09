use std::collections::HashMap;

use colored::Colorize;
use compositor_macros::{AvError, location, description};
use json_tree::{JSONPath, };

use crate::{core::error::{TraceableError, Traceable, AvError}, config::{config::{self, Config}, templating::{r#macro::SignatureMismatchError, AvMacro, avvalue::AvValue}}, Nadva::error::compare_errors};

pub trait ConfigurationSection : Sized {
    ///
    /// The absolute path to this section as a str.
    /// 
    const PATH : &'static str;

    ///
    /// Returns the absolute path to this section.
    /// Can be used in finding location of data.
    /// 
    fn path() -> JSONPath {
        Self::PATH.to_string().try_into().unwrap()
    }

    fn traceable(key : Option<bool>) -> Traceable {
        let loc = Config::index().get(&Self::path()).unwrap();
        Traceable::combine(&config::PATH.to_string(), loc, key)
    }

    fn from_map(declared : HashMap<AvMacro, AvValue>, raw : HashMap<String, serde_json::Value>) -> HashMap<AvMacro, AvValue> {
        let path : JSONPath = Self::path(); 

        let res = raw.iter().map(|(k, v)| {
            // Parse as a macro.
            let p = path.push(k.clone());
            let loc = Config::index().get(&p).unwrap();
            let k_pos = Traceable::combine(&Config::path(), loc, Some(true));
            let v_pos = Traceable::combine(&Config::path(), loc, Some(false));
            
            (
                AvMacro::parse(
                    k_pos.clone(),
                    k.clone()
                ),
                v,
                k_pos,
                v_pos
            )
        });

        // Syntactically invalid macros.

        let mut errors : Vec<(Box<dyn TraceableError>, Traceable)> = vec![]; 

        res
            .clone()
            .filter(|(k, _, _, _)| k.is_err())
            .for_each(|(k, _, p, _)| {
                let n = k.unwrap_err();
                
                for err in n {
                    errors.push((err, p.clone()));
                }
            }
            );
   

        // Syntactically Valid macros
        let defined = res
            .filter(|(k, _, _, _)| k.is_ok())
            .map(|(k, v, p1, p2)| (k.unwrap(), v, p1, p2));


        let mut output : HashMap<AvMacro, AvValue> = HashMap::new();
        
        
        let mut found_macros : Vec<usize> = vec![];
        // Look up the valid macros against our declared HashMap.

        for (declared_m, default_v) in declared {
            let defined_m  = defined.clone().position(|(m, _, _, _)| m.identifier() == declared_m.identifier());
            
            let (avmacro, avvalue) = match defined_m {
                None => {
                    // Macro not in user's config, 
                    // use default
                    // (and possibly issue a warning).

                    // TODO: @Sammy99jsp add 'not found' warning. 
                    (declared_m, default_v)
                },
                Some(i) => {
                    found_macros.push(i);
                    let (m, v, p, p_v) = defined.clone().nth(i).unwrap();

                    // Check if the macro's signature matches our defined one.
                    let sig_check = declared_m.has_signature(&m);

                    if let Err((delta, vec)) = sig_check {
                        errors.push((
                            Box::new(
                                SignatureMismatchError(
                                    p.clone(), 
                                    m.identifier(), 
                                    (delta, vec.iter().map(|e| (*e).clone()).collect())
                                ) 
                            ) as Box<dyn TraceableError>,
                            p
                        ));
                        
                        (declared_m, default_v)
                    } else {
                        // VALUE CHECKS
                        // Now check the value's type against the default's
                        match default_v
                            .parse_same_type(p_v.clone(), v.clone())
                        {
                            Err(e) => {
                                errors.push((e, p));
                                (declared_m, default_v)
                            },
                            Ok(val) => {
                                // Last value check:
                                // Check if value is consistent with macro
                                match val.consistent_with_macro(p_v.clone(), &m) {
                                    Ok(()) => (declared_m, val),
                                    Err(e) => {
                                        errors.push((e, p));
                                        (declared_m, default_v)
                                    },
                                }
                            }
                        }
                    }

                }
            };

            output.insert(avmacro, avvalue);
        }

        // User defined macros which were not found in our declaration.
        let not_found : Vec<_> = defined
            .enumerate()
            .filter(|(i, _)| !found_macros.contains(i))
            .map(|(_, e)| e)
            .collect();

        for (m, _, p1, _) in not_found {
            errors.push(
                (
                    Box::new(
                        MacroNotFound(p1.clone(), m.identifier(), Self::path())
                    ) as Box<dyn TraceableError>,
                    p1
                )
            )
        }

        errors
            .sort_by(|(a, _), (b, _)|
                compare_errors(a, b).unwrap()
            );
        
        for (err, _) in errors {
            println!("{}", err);
        }

        output
    }
}

#[AvError(TraceableError, CONFIG_MACRO_NOT_FOUND, "Config: Macro Not Found")]
pub struct MacroNotFound(pub Traceable, pub String, pub JSONPath);

impl TraceableError for MacroNotFound {
    location!(&self.0);
    description!(("The macro `{}` is not defined in this section (`{}`).", self.1.blue(), self.2));
}

