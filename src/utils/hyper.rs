use bytes::Bytes;
use http::{HeaderValue, Response, StatusCode};
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::header::{ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_ORIGIN};

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub fn accepted() -> Response<BoxBody<Bytes, hyper::Error>> {
    let mut resp = Response::new(full(""));
    *resp.status_mut() = StatusCode::ACCEPTED;
    resp.headers_mut()
        .append(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    resp.headers_mut().append(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization"),
    );
    resp
}

pub fn bad_request(str: String) -> Response<BoxBody<Bytes, hyper::Error>> {
    let mut resp = Response::new(full(str));
    *resp.status_mut() = StatusCode::BAD_REQUEST;
    resp.headers_mut()
        .append(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    resp
}
