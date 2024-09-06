//! A very basic HTTP server:
//! When it receives any request on port 25898, it will read the full request, ignore what's inside and then:
//! - Kill the ADB server
//! - Give a basic response saying that it has killed the ADB server.
//! - Wait a second
//! - Restart the ADB server.
//! 
//! This is used by the MBF site (in development mode only) to avoid a developer having to manually kill the ADB server whenever they want to use MBF.

use std::{io::{BufRead, BufReader, BufWriter, Write}, net::TcpListener, process::Command};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let listener = TcpListener::bind("localhost:25898")
        .context("Binding to port, is the mbf-adb-killer already running")?;

    loop {
        let (tcp_stream, addr) = listener.accept()?;

        let mut reader = BufReader::new(tcp_stream.try_clone()?);
        let mut writer = BufWriter::new(tcp_stream);

        println!("Got connection from {addr:?}");
        println!("Reading request");

        // Read all lines of the request until CRLFCRLF to mark its end
        let mut line_buf = String::new();
        while reader.read_line(&mut line_buf)? > 2 {
            line_buf.clear();
        }

        println!("Killing ADB");
        Command::new("adb").arg("kill-server").status()?;

        // Give a basic response so that javascript doesn't freak out.
        println!("Writing response");
        writer.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nContent-Type: text/html\r\nAccess-Control-Allow-Origin: *
\r\n\r\nKilled ADB server.")?;

        drop(reader);
        drop(writer);

        // Sleep for a second to allow MBF to connect
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Restart the ADB server so that the developer doesn't have to do this manually later, which takes a few seconds
        println!("Restarting ADB server");
        Command::new("adb").arg("start-server").status()?;
    }
}
