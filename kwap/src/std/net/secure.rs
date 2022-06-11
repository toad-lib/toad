use std::collections::HashMap;
use std::io;
use std::net::UdpSocket;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use kwap_common::prelude::*;

use super::{Socket, Addrd, convert::{ nb_to_io}};
use crate::todo::ResultExt2;

/// TODO
#[derive(Debug, Clone)]
pub struct UdpStream {
  sock: Arc<UdpSocket>,
  addr: no_std_net::SocketAddr,
  tx_buf: Vec<u8>,
}

impl UdpStream {
  fn new(sock: Arc<UdpSocket>, addr: no_std_net::SocketAddr) -> Self {
    Self { sock,
           addr,
           tx_buf: vec![] }
  }
}

impl io::Write for UdpStream {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.tx_buf = [&self.tx_buf, buf].concat();
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Socket::send(self.sock.as_ref(), Addrd(&self.tx_buf, self.addr)).map_err(nb_to_io)
  }
}

impl io::Read for UdpStream {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    let sock = self.sock.as_ref();
    let sock_ref = sock.deref();

    sock_ref.peek_addr()
            .validate(|rx_addr| {
              rx_addr.should_eq(&self.addr)
                     .else_err(|_| nb::Error::WouldBlock)
            })
            .bind(|_| Socket::recv(sock_ref, buf))
            .map_err(nb_to_io)
            .map(|Addrd(n, _)| n)
  }
}

/// TODO
#[derive(Debug)]
pub struct SecureUdpSocket {
  sock: Arc<UdpSocket>,
  streams: Mutex<HashMap<no_std_net::SocketAddr, Arc<Mutex<UdpStream>>>>,
}

impl SecureUdpSocket {
  /// TODO
  pub fn new(sock: UdpSocket) -> Self {
    Self { sock: Arc::new(sock),
           streams: Default::default() }
  }

  /// TODO
  pub(crate) fn get_stream(&self, addr: no_std_net::SocketAddr) -> Arc<Mutex<UdpStream>> {
    let mut streams = self.streams.lock().unwrap();
    let stream_ent = streams.entry(addr);

    stream_ent.or_insert(Arc::new(Mutex::new(UdpStream::new(self.sock.clone(), addr))))
              .clone()
  }
}

