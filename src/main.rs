use axum::{
    body::{Bytes, Full},
    extract::Path,
    handler::get,
    http::{header, HeaderValue, Request, Response, StatusCode},
    response::{Html, IntoResponse},
    Router,
};
use std::{convert::Infallible, net::SocketAddr};
use tokio::time::Duration;
use tower::{
    timeout::{error::Elapsed, TimeoutLayer},
    BoxError, Layer, Service,
};

#[tokio::main]
async fn main() {
    let svc = get(handler);

    let to = get(timeout_handler);

    // build our application with a route

    let app = Router::new()
        .route("/", svc)
        .route("/:timeout", to)
        .layer(LoggerLayer::new())
        .layer(TimeoutLayer::new(Duration::from_millis(300)))
        .handle_error(|error: BoxError| {
            println!("{:?}", error);
            if error.is::<Elapsed>() {
                return Ok::<_, Infallible>((
                    StatusCode::REQUEST_TIMEOUT,
                    Html("<h1>Request took too long</h1>").into_response(),
                ));
            }

            Ok::<_, Infallible>((
                StatusCode::INTERNAL_SERVER_ERROR,
                // `Cow` lets us return either `&str` or `String`
                error_response(format!("<h1>Unhandled internal error</h1><p>{}</p>", error)),
            ))
        });

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        //.serve(make_svc)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn timeout_handler(Path(id): Path<u64>) -> Html<&'static str> {
    tokio::time::sleep(Duration::from_millis(id)).await;
    Html("<h1>Made it!</h1>")
}

fn error_response(str: String) -> Response<Full<Bytes>> {
    let mut res = Response::new(str.into());
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html"));
    res
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

#[derive(Clone, Copy)]
struct Logger<S> {
    inner: S,
}

impl<S> Logger<S> {
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, B> Service<Request<B>> for Logger<S>
where
    S: Service<Request<B>> + Clone + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        println!("processing {} {}", req.method(), req.uri().path());
        self.inner.call(req)
    }
}

#[derive(Clone, Copy)]
struct LoggerLayer;

impl LoggerLayer {
    fn new() -> Self {
        Self {}
    }
}

impl<S> Layer<S> for LoggerLayer {
    type Service = Logger<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Logger::new(inner)
    }
}
