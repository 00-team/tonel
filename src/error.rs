use teloxide::RequestError;

#[derive(Debug)]
pub enum Worm {
    TxRq(RequestError),
}

#[derive(Debug)]
pub struct AppErr {
    pub worm: Worm,
    pub debug: String,
}

impl From<RequestError> for AppErr {
    fn from(value: RequestError) -> Self {
        Self { debug: value.to_string(), worm: Worm::TxRq(value) }
    }
}
