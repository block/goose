use std::future::Future;

pub struct Error;

pub struct Model {
    pub name: String,
}

pub struct StreamingRequest {
    pub model: Model,
}

pub struct StreamingResponse;

pub trait Provider {
    fn stream(req: StreamingRequest) -> impl Future<Output = Result<StreamingResponse, Error>>;
}
