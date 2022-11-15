use std::future::{ready, Ready};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, Error
};
use futures_util::future::LocalBoxFuture;
use slog_scope::info;

pub struct Logger;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for Logger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LoggerMiddleware { service }))
    }
}

pub struct LoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for LoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = std::time::Instant::now();
        let path = req.uri().to_string();
        let method = req.method().to_string();
        let req_ip = req.connection_info().realip_remote_addr().unwrap_or("Unknown IP").to_string();
        let req_ver = req.version();
        let user_agent = req
            .headers()
            .get("user-agent")
            .map(|v| v.to_str().unwrap_or("unknown"))
            .unwrap_or("unknown")
            .to_string();
        let req_id = crate::models::create_token(12);

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            let elapsed = start.elapsed();

            let http_resp = res.response();

            // TODO: Port size when we can do so
            info!(
                "Got Request";
                "status" => http_resp.status().as_u16(),
                "statusText" => http_resp.status().canonical_reason(),
                "method" => method,
                "url" => path,
                "reqIp" => req_ip,
                "size" => "Not Implemented",
                "protocol" => format!("{:?}", req_ver),
                "latency" => elapsed.as_millis(),
                "userAgent" => user_agent,
                "reqId" => req_id,
            );

            Ok(res)
        })
    }
}

