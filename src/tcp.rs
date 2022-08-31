#![allow(unused)]

use core::ffi::{c_void};
use crate::framer::Stream;
use core::convert::From;
use core::ptr::null_mut;

const DEFAULT_TIME:u32 = 500;

#[inline]
fn get_errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

#[derive(Debug)]
pub enum IoError {
    Timeout,
    Other(i32)
}

impl From<i32> for IoError {
    fn from(err_no: i32) -> Self {
        match err_no {
            libc::EBUSY | libc::EWOULDBLOCK => IoError::Timeout,
            e => IoError::Other(e),
        }
    }
}

pub struct TcpStream {
    fd: i32,
}
impl TcpStream {
    pub fn connect(ip_host: u32, port:u16) -> Result<Self, IoError> {
        unsafe {
            let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, libc::IPPROTO_TCP);
            if sock <0 {
                return Err(IoError::from(get_errno()));
            }
            let my = Self{ fd: sock};
            my.connect_internal(ip_host, port)?;
            my.set_send_timeout(DEFAULT_TIME)?;
            my.set_recv_timeout(DEFAULT_TIME)?;
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
            return Err(IoError::from(get_errno()));
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
            return Err(IoError::from(get_errno()));
        }
        Ok(())
    }

    // @return: 0: timeout, >0: has data, <0:error
    fn wait_event(&self, is_write: bool, to:u32) -> i32 {
        unsafe {
            let mut sets: libc::fd_set = core::mem::zeroed();
            libc::FD_SET(self.fd, &mut sets);
            let mut tv = libc::timeval {
                tv_sec: (to/1000) as libc::time_t,
                tv_usec: (to%1000*1000) as libc::suseconds_t,
            };
            let ret = if is_write {
                libc::select(self.fd +1, null_mut(), &mut sets, null_mut(), &mut tv)
            } else {
                libc::select(self.fd +1, &mut sets, null_mut(), null_mut(), &mut tv)
            };
            ret
        }
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
            return Err(IoError::from(get_errno()));
        }
        Ok(len as usize)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), IoError> {
        let len = unsafe{ libc::send(self.fd, buf.as_ptr() as *const c_void, buf.len() as libc::size_t, 0)};
        if len <0 {
            return Err(IoError::from(get_errno()));
        }
        Ok(())
    }
}
