// The MIT License (MIT)
// Copyright (c) 2019 David Haig

// Demo websocket client connecting to localhost port 1337.
// This will initiate a websocket connection to path /chat. The demo sends a simple "Hello, World!"
// message and expects an echo of the same message as a reply.
// It will then initiate a close handshake, wait for a close response from the server,
// and terminate the connection.
// Note that we are using the standard library in the demo and making use of the framer helper module
// but the websocket library remains no_std (see client_full for an example without the framer helper module)
#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(default_alloc_error_handler)]
extern crate alloc;

use embedded_websocket::{
    framer::{Framer, ReadResult, FramerError},
    WebSocketClient, WebSocketCloseStatusCode, WebSocketOptions, WebSocketSendMessageType,
};
use embedded_websocket::tcp::{TcpStream, IoError};
use core::ptr::null_mut;
use alloc::format;

#[no_mangle]
pub extern "C" fn main(_argc:i32, _argv: *mut *mut i8) -> i32 {
    // open a TCP stream to localhost port 1337
    // let address = "127.0.0.1:1337";
    // println!("Connecting to: {}", address);
    let mut stream = TcpStream::connect(0x7f000001, 1337).unwrap();
    unsafe{ libc::printf("Connected.\n\0".as_ptr() as *const i8) };

    let mut read_buf = [0; 4000];
    let mut read_cursor = 0;
    let mut write_buf = [0; 4000];
    let mut frame_buf = [0; 4000];
    let mut websocket = WebSocketClient::new_client(rand::thread_rng());

    // initiate a websocket opening handshake
    let websocket_options = WebSocketOptions {
        path: "/chat",
        host: "localhost",
        origin: "http://localhost:1337",
        sub_protocols: None,
        additional_headers: None,
    };

    let mut framer = Framer::new(
        &mut read_buf,
        &mut read_cursor,
        &mut write_buf,
        &mut websocket,
    );
    framer.connect(&mut stream, &websocket_options).unwrap();

    let message = ">>>";
    framer.write(
        &mut stream,
        WebSocketSendMessageType::Text,
        true,
        message.as_bytes(),
    ).unwrap();

    let mut count = 0;
    loop {
        match framer.read(&mut stream, &mut frame_buf) {
            Ok(ReadResult::Text(s)) =>     {
                unsafe{ libc::printf("[%u] Received:%s\n\0".as_ptr() as *const i8, libc::time(null_mut()), s.as_ptr()) };
            }
            Ok(_e) => {
                panic!("other read result!");
            }
            Err(FramerError::Io(IoError::Timeout)) => {
                unsafe{ libc::printf("[%u] Received timeout\n\0".as_ptr() as *const i8, libc::time(null_mut())) };
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
        // unsafe { libc::usleep(1_000_000) };
        count += 1;
        if count > 20 {
            // close the websocket after receiving the first reply
            framer.close(&mut stream, WebSocketCloseStatusCode::NormalClosure, None).unwrap();
            unsafe{ libc::printf("Sent close handshake\n\0".as_ptr() as *const i8) };
            break;
        }
        if count % 2 == 0 {
            let msg = format!("text {}", count);
            framer.write(
                &mut stream,
                WebSocketSendMessageType::Text,
                true,
                msg.as_bytes(),
            ).unwrap();
        }
    }
    unsafe{ libc::printf("Connection closed\n\0".as_ptr() as *const i8) };
    0
}
