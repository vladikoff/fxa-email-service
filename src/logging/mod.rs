// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

use std::{io, ops::Deref};

use failure::err_msg;
use rocket::{Request, State};
use slog::{self, Drain, Record, Serializer, KV};
use slog_async;
use slog_mozlog_json::MozLogJson;
use slog_term;

use app_errors::AppResult;
use settings::Settings;

lazy_static! {
    static ref LOGGER_NAME: String =
        format!("{}-{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    static ref MSG_TYPE: String = format!("{}:log", env!("CARGO_PKG_NAME"));
}

#[derive(Clone)]
struct RequestMozlogFields {
    method: &'static str,
    path: String,
    remote: Option<String>,
    agent: Option<String>,
}

impl RequestMozlogFields {
    pub fn from_request(request: &Request) -> RequestMozlogFields {
        let headers = request.headers();
        RequestMozlogFields {
            method: request.method().as_str(),
            path: request.uri().to_string(),
            agent: headers.get_one("User-Agent").map(&str::to_owned),
            remote: headers
                .get_one("X-Forwarded-For")
                .map(&str::to_owned)
                .or(request.remote().map(|addr| addr.ip().to_string())),
        }
    }
}

impl KV for RequestMozlogFields {
    fn serialize(&self, _: &Record, serializer: &mut Serializer) -> slog::Result {
        if let Some(ref agent) = self.agent {
            serializer.emit_str("agent", agent)?;
        }
        if let Some(ref remote) = self.remote {
            serializer.emit_str("remoteAddressChain", remote)?;
        }
        serializer.emit_str("path", &self.path)?;
        serializer.emit_str("method", self.method)?;

        Ok(())
    }
}

pub struct MozlogLogger(slog::Logger);

impl MozlogLogger {
    pub fn new(settings: &Settings) -> AppResult<MozlogLogger> {
        let logger = if settings.mozlog {
            let drain = MozLogJson::new(io::stdout())
                .logger_name(LOGGER_NAME.to_owned())
                .msg_type(MSG_TYPE.to_owned())
                .build()
                .fuse();
            let drain = slog_async::Async::new(drain).build().fuse();
            slog::Logger::root(drain, slog_o!())
        } else {
            let decorator = slog_term::TermDecorator::new().build();
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            let drain = slog_async::Async::new(drain).build().fuse();
            slog::Logger::root(drain, slog_o!())
        };

        Ok(MozlogLogger(logger))
    }

    pub fn with_request(request: &Request) -> AppResult<MozlogLogger> {
        let logger = request
            .guard::<State<MozlogLogger>>()
            .success_or(err_msg("Internal error: No managed MozlogLogger"))?;
        Ok(MozlogLogger(
            logger.new(slog_o!(RequestMozlogFields::from_request(request))),
        ))
    }
}

impl Deref for MozlogLogger {
    type Target = slog::Logger;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
