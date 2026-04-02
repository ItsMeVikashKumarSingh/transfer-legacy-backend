use tower::Layer;

#[derive(Clone, Default)]
pub struct SentryLayer;

impl SentryLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for SentryLayer {
    type Service = S;

    fn layer(&self, service: S) -> Self::Service {
        service
    }
}
