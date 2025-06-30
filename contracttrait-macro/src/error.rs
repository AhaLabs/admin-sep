use proc_macro2::TokenStream;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Stream(TokenStream),
    #[error(transparent)]
    Syn(#[from] syn::Error),
}

impl From<Error> for TokenStream {
    fn from(value: Error) -> Self {
        match value {
            Error::Stream(ts) => ts,
            Error::Syn(error) => error.to_compile_error(),
        }
    }
}
