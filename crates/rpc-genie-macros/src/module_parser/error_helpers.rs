pub fn combine_errors(errors: &mut Option<syn::Error>, error: syn::Error) {
    match errors {
        Some(errors) => errors.combine(error),
        None => *errors = Some(error),
    }
}

pub fn create_result<T>(ok: T, maybe_errors: Option<syn::Error>) -> syn::Result<T> {
    match maybe_errors {
        Some(err) => Err(err),
        None => Ok(ok),
    }
}
