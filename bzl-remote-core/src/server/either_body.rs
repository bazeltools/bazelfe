use std::{
    pin::Pin,
    task::{Context, Poll},
};

use super::BoxError;

pub enum EitherBody<A, B> {
    Left(A),
    Right(B),
}

impl<A, B> http_body::Body for EitherBody<A, B>
where
    A: http_body::Body + Send + Unpin,
    B: http_body::Body<Data = A::Data> + Send + Unpin,
    A::Error: Into<BoxError>,
    B::Error: Into<BoxError>,
{
    type Data = A::Data;
    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

    fn is_end_stream(&self) -> bool {
        match self {
            EitherBody::Left(b) => b.is_end_stream(),
            EitherBody::Right(b) => b.is_end_stream(),
        }
    }

    fn poll_frame(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
          match self.get_mut() {
            EitherBody::Left(b) => Pin::new(b).poll_frame(cx).map_err(Into::into),
            EitherBody::Right(b) => Pin::new(b).poll_frame(cx).map_err(Into::into),
        }
    }
}
