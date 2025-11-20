use proc_macro::TokenStream;
use proc_macro2::Span;
use std::fmt::Write;
use syn::{
    parse_macro_input, parse_quote, Error, FnArg, Ident, ItemFn, Pat, PathArguments, ReturnType,
    Type,
};

use quote::quote;

use culpa::{throw, throws};

fn ty_is_borrow_str(ty: &Type) -> bool {
    if let Type::Reference(ty) = ty {
        if ty.mutability.is_none() && ty.lifetime.is_none() {
            if let Type::Path(pp) = &*ty.elem {
                pp.path.is_ident("str")
            } else {
                // not a path, so not &str
                false
            }
        } else {
            // mutable, or a lifetime stated, so not &str
            false
        }
    } else {
        // Not & so not &str
        false
    }
}

fn ty_is_borrow_path(ty: &Type) -> bool {
    if let Type::Reference(ty) = ty {
        if ty.mutability.is_none() && ty.lifetime.is_none() {
            if let Type::Path(pp) = &*ty.elem {
                pp.path.is_ident("Path")
            } else {
                // not a path, so not &Path
                false
            }
        } else {
            // mutable, or a lifetime stated, so not &Path
            false
        }
    } else {
        // Not & so not &Path
        false
    }
}

fn ty_is_datafile(ty: &Type) -> bool {
    if let Type::Path(ty) = ty {
        ty.path.is_ident("SubplotDataFile")
    } else {
        false
    }
}

fn ty_is_scenariocontext(ty: &Type) -> bool {
    if let Type::Path(ty) = ty {
        ty.path.is_ident("ScenarioContext")
    } else {
        false
    }
}

#[throws(Error)]
fn ty_as_path(ty: &Type) -> String {
    if let Type::Path(p) = ty {
        let mut ret = String::new();
        let mut colons = p.path.leading_colon.is_some();
        for seg in &p.path.segments {
            if !matches!(seg.arguments, PathArguments::None) {
                throw!(Error::new_spanned(seg, "unexpected path segment arguments"));
            }
            if colons {
                ret.push_str("::");
            }
            colons = true;
            ret.push_str(&seg.ident.to_string());
        }
        ret
    } else {
        throw!(Error::new_spanned(ty, "expected a type path"));
    }
}

#[throws(Error)]
fn check_step_declaration(step: &ItemFn) {
    // Step functions must be declared very simply as:
    // fn stepfunctionname(context: &mut Context)
    // the `mut` is optional, but the type of the context argument must
    // be a borrow of some kind, its name is not important.
    // If the step function takes any arguments, then they must come next
    // and should be named in the usual way.  If the argument starts with
    // an underscore then that will be stripped during argument conversion
    // so that if you're just ignoring an argument from your step you can.
    // Additionally, step functions must **NOT** have a return type declared
    // and must not be generic, non-rust ABI, unsafe, etc. in any way.
    // Finally const makes no sense, though we won't deny it for now.
    // Visibility will be taken into account when constructing the associated
    // content for the step
    let sig = &step.sig;
    if let Some(syncness) = sig.asyncness.as_ref() {
        throw!(Error::new_spanned(
            syncness,
            "Step functions may not be async",
        ));
    }
    if let Some(unsafeness) = sig.unsafety.as_ref() {
        throw!(Error::new_spanned(
            unsafeness,
            "Step functions may not be unsafe",
        ));
    }
    if let Some(abi) = sig.abi.as_ref() {
        throw!(Error::new_spanned(
            abi,
            "Step functions may not specify an ABI",
        ));
    }
    if !matches!(sig.output, ReturnType::Default) {
        throw!(Error::new_spanned(
            &sig.output,
            "Step functions may not specify a return value",
        ));
    }
    if let Some(variadic) = sig.variadic.as_ref() {
        throw!(Error::new_spanned(
            variadic,
            "Step functions may not be variadic",
        ));
    }
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        throw!(Error::new_spanned(
            &sig.generics,
            "Step functions may not be generic",
        ));
    }
    if let Some(arg) = sig.inputs.first() {
        if let FnArg::Typed(pat) = arg {
            if let Type::Reference(tr) = &*pat.ty {
                if let Some(lifetime) = tr.lifetime.as_ref() {
                    throw!(Error::new_spanned(
                        lifetime,
                        "Step function context borrow should not be given a lifetime marker",
                    ));
                }
            } else {
                throw!(Error::new_spanned(
                    pat,
                    "Step function context must be taken as a borrow",
                ));
            }
        } else {
            throw!(Error::new_spanned(
                arg,
                "Step functions do not take a method receiver",
            ));
        }
    } else {
        throw!(Error::new_spanned(
            &sig.inputs,
            "Step functions must have at least 1 argument (context)",
        ));
    }
}

#[throws(Error)]
fn process_step(mut input: ItemFn) -> proc_macro2::TokenStream {
    // Processing a step involves constructing a step builder for
    // the function which returns a step object to be passed into the
    // scenario system

    // A step builder consists of a struct whose fields are of the
    // appropriate type, a set of pub methods to set those fields
    // and then a build call which constructs the step instance with
    // an appropriate closure in it

    let vis = input.vis.clone();
    let stepname = input.sig.ident.clone();
    let mutablectx = {
        if let FnArg::Typed(pt) = &input.sig.inputs[0] {
            if let Type::Reference(pp) = &*pt.ty {
                pp.mutability.is_some()
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
    };

    let contexttype = if let Some(ty) = input.sig.inputs.first() {
        match ty {
            FnArg::Typed(pt) => {
                if let Type::Reference(rt) = &*pt.ty {
                    *(rt.elem).clone()
                } else {
                    unreachable!()
                }
            }
            _ => unreachable!(),
        }
    } else {
        unreachable!()
    };

    let contexts: Vec<Type> = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("context"))
        .map(|attr| {
            let ty: Type = attr.parse_args()?;
            Ok(ty)
        })
        .collect::<Result<_, Error>>()?;

    input.attrs.retain(|f| !f.path().is_ident("context"));

    let docs: Vec<_> = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();

    let fields = input
        .sig
        .inputs
        .iter()
        .skip(1)
        .map(|a| {
            if let FnArg::Typed(pat) = a {
                if let Pat::Ident(ident) = &*pat.pat {
                    if let Some(r) = ident.by_ref.as_ref() {
                        Err(Error::new_spanned(r, "ref not valid here"))
                    } else if let Some(subpat) = ident.subpat.as_ref() {
                        Err(Error::new_spanned(&subpat.1, "subpattern not valid here"))
                    } else {
                        let identstr = ident.ident.to_string();
                        Ok((
                            Ident::new(identstr.trim_start_matches('_'), ident.ident.span()),
                            (*pat.ty).clone(),
                        ))
                    }
                } else {
                    Err(Error::new_spanned(pat, "expected a simple name here"))
                }
            } else {
                Err(Error::new_spanned(
                    a,
                    "receiver argument unexpected in this position",
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let structdef = {
        let structfields: Vec<_> = fields
            .iter()
            .map(|(id, ty)| {
                let ty = if ty_is_borrow_str(ty) {
                    parse_quote!(::std::string::String)
                } else if ty_is_borrow_path(ty) {
                    parse_quote!(::std::path::PathBuf)
                } else {
                    ty.clone()
                };
                quote! {
                    #id : #ty
                }
            })
            .collect();
        quote! {
            #[allow(non_camel_case_types)]
            #[allow(unused)]
            #[derive(Default)]
            #[doc(hidden)]
            pub struct Builder {
                #(#structfields),*
            }
        }
    };

    let withfn = if mutablectx {
        Ident::new("with_mut", Span::call_site())
    } else {
        Ident::new("with", Span::call_site())
    };

    let structimpl = {
        let fieldfns: Vec<_> = fields
            .iter()
            .map(|(id, ty)| {
                if ty_is_borrow_str(ty) {
                    quote! {
                        pub fn #id(mut self, value: &str) -> Self {
                            self.#id = value.to_string();
                            self
                        }
                    }
                } else if ty_is_borrow_path(ty) {
                    quote! {
                        pub fn #id<P: Into<std::path::PathBuf>>(mut self, value: P) -> Self {
                            self.#id = value.into();
                            self
                        }
                    }
                } else {
                    quote! {
                        pub fn #id(mut self, value: #ty) -> Self {
                            self.#id = value;
                            self
                        }
                    }
                }
            })
            .collect();

        let buildargs: Vec<_> = fields
            .iter()
            .map(|(id, ty)| {
                if ty_is_borrow_str(ty) || ty_is_borrow_path(ty) {
                    quote! {
                       &self.#id
                    }
                } else if ty_is_datafile(ty) {
                    quote! {
                        self.#id.clone()
                    }
                } else {
                    quote! {
                        self.#id
                    }
                }
            })
            .collect();

        let builder_body = if ty_is_scenariocontext(&contexttype) {
            quote! {
                #stepname(ctx,#(#buildargs),*)
            }
        } else {
            quote! {
                ctx.#withfn (|ctx| #stepname(ctx, #(#buildargs),*), _defuse_poison)
            }
        };

        quote! {
            impl Builder {
                #(#fieldfns)*

                pub fn build(self, step_text: String, location: &'static str) -> ScenarioStep {
                    ScenarioStep::new(step_text, move |ctx, _defuse_poison|
                        #builder_body,
                        |scenario| register_contexts(scenario),
                        location,
                    )
                }
            }
        }
    };

    let inputargs: Vec<_> = fields.iter().map(|(i, t)| quote!(#i : #t)).collect();
    let argnames: Vec<_> = fields.iter().map(|(i, _)| i).collect();

    let call_body = if ty_is_scenariocontext(&contexttype) {
        quote! {
            #stepname(___context___,#(#argnames),*)
        }
    } else {
        quote! {
            ___context___.#withfn (move |ctx| #stepname(ctx, #(#argnames),*),false)
        }
    };

    let extra_registers: Vec<_> = contexts
        .iter()
        .map(|ty| {
            quote! {
                scenario.register_context_type::<#ty>();
            }
        })
        .collect();

    let register_fn_body = if ty_is_scenariocontext(&contexttype) {
        quote! {
            #(#extra_registers)*
        }
    } else {
        quote! {
            scenario.register_context_type::<#contexttype>();
            #(#extra_registers)*
        }
    };

    let call_docs = {
        let mut contextattrs = String::new();
        let outer_ctx = if ty_is_scenariocontext(&contexttype) {
            None
        } else {
            Some(&contexttype)
        };
        for context in outer_ctx.into_iter().chain(contexts.iter()) {
            write!(contextattrs, "\n    #[context({:?})]", ty_as_path(context)?).unwrap();
        }
        let func_args: Vec<_> = fields.iter().map(|(ident, _)| format!("{ident}")).collect();
        let func_args = func_args.join(", ");
        format!(
            r#"
    Call [this step][self] function from another.

    If you want to call this step function from another, you will
    need to do something like this:

    ```rust,ignore
    #[step]{contextattrs}
    fn defer_to_{stepname}(context: &ScenarioContext) {{
        //...
        {stepname}::call(context, {func_args})?;
        // ...
    }}
    ```
    "#,
        )
    };
    let ret = quote! {
        #(#docs)*
        #vis mod #stepname {
            use super::*;
            pub(crate) use super::#contexttype;

            #structdef
            #structimpl

            #[throws(StepError)]
            #[allow(dead_code)] // It's okay for step functions to not be used
            #[deny(unused_must_use)]
            #[doc(hidden)]
            #input

            #[doc = #call_docs]
            pub fn call(___context___: &ScenarioContext, #(#inputargs),*) -> StepResult {
                #call_body
            }

            #[allow(unused_variables)]
            #[doc(hidden)]
            pub fn register_contexts(scenario: &Scenario) {
                #register_fn_body
            }
        }
    };

    ret
}

#[proc_macro_attribute]
pub fn step(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    if let Err(e) = check_step_declaration(&input) {
        return e.to_compile_error().into();
    }

    match process_step(input) {
        Ok(toks) => toks.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
