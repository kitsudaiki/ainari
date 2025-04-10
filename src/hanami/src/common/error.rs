#[derive(Debug)]
pub enum ErrorType {
    Error = 0,
    InvalidInput = 1,
}

#[derive(Debug)]
pub struct ErrorContainer {
    pub error_type: ErrorType,
    pub msg: String,
}

