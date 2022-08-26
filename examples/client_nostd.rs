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

use core::ffi::{c_void};
use embedded_websocket::{
    framer::{Framer, ReadResult},
    WebSocketClient, WebSocketCloseStatusCode, WebSocketOptions, WebSocketSendMessageType,
};
use embedded_websocket::framer::Stream;
use alloc::string::String;
use alloc::format;

type IoError = String;

const DEFAULT_TIME:u32 = 5000;

struct TcpStream {
    fd: i32,
}
impl TcpStream {
    pub fn connect(ip_host: u32, port:u16) -> Result<Self, IoError> {
        unsafe {
            let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_TCP);
            if sock <0 {
                return Err(format!("create socket failed: {}", sock));
            }
            let my = Self{ fd: sock};
            my.set_send_timeout(DEFAULT_TIME)?;
            my.set_recv_timeout(DEFAULT_TIME)?;
            my.connect_internal(ip_host, port)?;
            Ok(my)
        }
    }

    pub fn connect_internal(&self, ip_host:u32, port:u16) -> Result<(), IoError> {
        let addr = libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: libc::in_addr {
                s_addr: ip_host.to_be(),
            },
            sin_zero: [0u8; 8],
        };
        let ret = unsafe{ libc::connect(self.fd, &addr as *const libc::sockaddr_in as *const libc::sockaddr, core::mem::size_of::<libc::sockaddr_in>() as u32) };
        if ret != 0 {
            return Err(format!("connect failed: {}", ret));
        }
        Ok(())
    }
    pub fn set_send_timeout(&self, to:u32) -> Result<(), IoError> {
        self.set_timeout(libc::SO_SNDTIMEO, to)
    }
    pub fn set_recv_timeout(&self, to:u32) -> Result<(), IoError> {
        self.set_timeout(libc::SO_RCVTIMEO, to)
    }

    fn set_timeout(&self, name: i32, to:u32) -> Result<(), IoError> {
        let tv = libc::timeval {
            tv_sec: (to/1000) as libc::time_t,
            tv_usec: (to%1000*1000) as libc::suseconds_t,
        };
        let ret = unsafe{ libc::setsockopt(self.fd, libc::SOL_SOCKET, name, &tv as *const libc::timeval as *const c_void, core::mem::size_of::<libc::timeval>() as libc::socklen_t) };
        if ret != 0 {
            return Err(format!("set timeout failed: {}", ret));
        }
        Ok(())
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        if self.fd >=0 {
            unsafe{ libc::close(self.fd) };
        }
    }
}
impl Stream<IoError> for TcpStream  {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError> {
        let len = unsafe{ libc::recv(self.fd, buf.as_mut_ptr() as *mut c_void, buf.len() as libc::size_t, 0)};
        if len <0 {
            return Err(format!("read failed: {}", len));
        }
        Ok(len as usize)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), IoError> {
        let len = unsafe{ libc::send(self.fd, buf.as_ptr() as *const c_void, buf.len() as libc::size_t, 0)};
        if len <0 {
            return Err(format!("write failed: {}", len));
        }
        Ok(())
    }
}

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

    let message = "Hello, World!";
    framer.write(
        &mut stream,
        WebSocketSendMessageType::Text,
        true,
        message.as_bytes(),
    ).unwrap();

    while let ReadResult::Text(s) = framer.read(&mut stream, &mut frame_buf).unwrap() {
        unsafe{ libc::printf("Received:%s\n\0".as_ptr() as *const i8, s.as_ptr()) };

        // close the websocket after receiving the first reply
        framer.close(&mut stream, WebSocketCloseStatusCode::NormalClosure, None).unwrap();
        unsafe{ libc::printf("Sent close handshake\n\0".as_ptr() as *const i8) };
    }

    unsafe{ libc::printf("Connection closed\n\0".as_ptr() as *const i8) };
    0
}
