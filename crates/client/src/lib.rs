//! Blocking client for the control channel of `tunelock-service`, shared by
//! the CLI and the desktop app.

use std::io::{BufRead, BufReader, Read, Write};

use anyhow::{Context, bail};
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericNamespaced, Stream};
use tunelock_core::Allowlist;
use tunelock_protocol::{
    MAX_LINE_LEN, PROTOCOL_VERSION, Request, Response, Status, decode_line, encode_line,
};

/// A blocking connection to the service's control socket.
pub struct Client {
    stream: BufReader<Stream>,
}

impl Client {
    /// Connects to the control socket `name`, usually
    /// [`tunelock_protocol::CONTROL_SOCKET`].
    ///
    /// # Errors
    /// If the service is not running or the name is invalid.
    pub fn connect(name: &str) -> anyhow::Result<Self> {
        let name = name.to_ns_name::<GenericNamespaced>()?;
        let stream = Stream::connect(name)
            .context("could not reach the Tunelock service; is it running?")?;
        Ok(Self {
            stream: BufReader::new(stream),
        })
    }

    /// Sends one request and waits for its response. A [`Response::Error`]
    /// becomes an `Err` with the service's message.
    ///
    /// # Errors
    /// On I/O errors, malformed responses or an error from the service.
    pub fn send(&mut self, request: &Request) -> anyhow::Result<Response> {
        self.stream
            .get_mut()
            .write_all(encode_line(request)?.as_bytes())?;
        let mut line = String::new();
        let limit = u64::try_from(MAX_LINE_LEN).unwrap_or(u64::MAX) + 1;
        (&mut self.stream).take(limit).read_line(&mut line)?;
        if line.is_empty() {
            bail!("the service closed the connection");
        }
        let response = decode_line(&line).context("unexpected response from the service")?;
        if let Response::Error(error) = response {
            bail!("{}", error.message);
        }
        Ok(response)
    }

    /// Sends a request that only acknowledges with `Done`.
    ///
    /// # Errors
    /// As [`Client::send`], or if the response is not `Done`.
    pub fn done(&mut self, request: &Request) -> anyhow::Result<()> {
        match self.send(request)? {
            Response::Done => Ok(()),
            other => bail!("unexpected response: {other:?}"),
        }
    }

    /// # Errors
    /// As [`Client::send`], or if the service speaks another protocol version.
    pub fn status(&mut self) -> anyhow::Result<Status> {
        let Response::Status(status) = self.send(&Request::Status)? else {
            bail!("unexpected response");
        };
        if status.protocol_version != PROTOCOL_VERSION {
            bail!(
                "service protocol v{} does not match this client (v{PROTOCOL_VERSION}); update both",
                status.protocol_version
            );
        }
        Ok(status)
    }

    /// The saved allowlist, if any.
    ///
    /// # Errors
    /// As [`Client::send`].
    pub fn allowlist(&mut self) -> anyhow::Result<Option<Allowlist>> {
        let Response::Allowlist { allowlist } = self.send(&Request::GetAllowlist)? else {
            bail!("unexpected response");
        };
        Ok(allowlist)
    }
}
