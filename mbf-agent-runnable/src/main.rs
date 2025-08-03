use std::{io::{BufRead, BufReader, Write}, panic};

use anyhow::{Context, Result};
use log::{error, Level};
use mbf_agent_core::{handlers, models::{request, response}, parameters::init_parameters};

static LOGGER: ResponseLogger = ResponseLogger {};

struct ResponseLogger {}

impl log::Log for ResponseLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &log::Record) {
        // Skip logs that are not from mbf_agent, mbf_zip, etc.
        // ...as these are spammy logs from ureq or rustls, and we do nto want them.
        match record.module_path() {
            Some(module_path) => {
                if !module_path.starts_with("mbf") {
                    return;
                }
            }
            None => return,
        }

        // Ignore errors, logging should be infallible and we don't want to panic
        let _result = write_response(response::Response::LogMsg {
            message: format!("{}", record.args()),
            level: match record.level() {
                Level::Debug => response::LogLevel::Debug,
                Level::Info => response::LogLevel::Info,
                Level::Warn => response::LogLevel::Warn,
                Level::Error => response::LogLevel::Error,
                Level::Trace => response::LogLevel::Trace,
            },
        });
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

fn write_response(response: response::Response) -> Result<()> {
    let mut lock = std::io::stdout().lock();
    serde_json::to_writer(&mut lock, &response).context("Serializing JSON response")?;
    writeln!(lock)?;
    Ok(())
}

fn main() -> Result<()> {
    #[cfg(feature = "request_timing")]
    let start_time = Instant::now();

    log::set_logger(&LOGGER).expect("Failed to set up logging");
    log::set_max_level(log::LevelFilter::Debug);

    let mut reader = BufReader::new(std::io::stdin());
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let req: request::Request = serde_json::from_str(&line)?;

    // Set the parameters for this instance of the agent
    init_parameters(
        &req.agent_parameters.game_id,
        req.agent_parameters.ignore_package_id,
    );

    // Set a panic hook that writes the panic as a JSON Log
    // (we don't do this in catch_unwind as we get an `Any` there, which doesn't implement Display)
    panic::set_hook(Box::new(|info| {
        error!("Request failed due to a panic!: {info}")
    }));

    match std::panic::catch_unwind(|| handlers::handle_request(req)) {
        Ok(resp) => match resp {
            Ok(resp) => {
                #[cfg(feature = "request_timing")]
                {
                    let req_time = Instant::now() - start_time;
                    info!("Request complete in {}ms", req_time.as_millis());
                }

                write_response(resp)?;
            }
            Err(err) => error!("{err:?}"),
        },
        Err(_) => {} // Panic will be outputted above
    };

    Ok(())
}
