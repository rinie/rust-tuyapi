use crate::error::ErrorKind;
use crate::mesparse::{CommandType, Message, MessageParser, Result};
use log::{debug, info};
use std::io::prelude::*;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

pub struct TuyaDevice {
    mp: MessageParser,
    addr: SocketAddr,
}

impl TuyaDevice {
    pub fn create(ver: &str, key: Option<&str>, addr: IpAddr) -> Result<TuyaDevice> {
        let mp = MessageParser::create(ver, key)?;
        Ok(TuyaDevice::create_with_mp(mp, addr))
    }

    pub fn create_with_mp(mp: MessageParser, addr: IpAddr) -> TuyaDevice {
        TuyaDevice {
            mp,
            addr: SocketAddr::new(addr, 6668),
        }
    }

    pub fn set(&self, tuya_payload: &str, seq_id: u32) -> Result<()> {
        let mes = Message::new(tuya_payload.as_bytes(), CommandType::Control, Some(seq_id));
        let replies = self.send(&mes, tuya_payload, seq_id)?;
        replies
            .iter()
            .for_each(|mes| info!("Decoded response ({}):\n{}", seq_id, mes));
        Ok(())
    }

    pub fn get(&self, tuya_payload: &str, seq_id: u32) -> Result<Vec<Message>> {
        let mes = Message::new(tuya_payload.as_bytes(), CommandType::DpQuery, Some(seq_id));
        let replies = self.send(&mes, tuya_payload, seq_id)?;
        replies
            .iter()
            .for_each(|mes| info!("Decoded response ({}):\n{}", seq_id, mes));
        Ok(replies)
    }

    fn send(&self, mes: &Message, payload: &str, seq_id: u32) -> Result<Vec<Message>> {
        let mut tcpstream = TcpStream::connect(&self.addr).map_err(ErrorKind::TcpError)?;
        tcpstream.set_nodelay(true).map_err(ErrorKind::TcpError)?;
        tcpstream
            .set_read_timeout(Some(Duration::new(2, 0)))
            .map_err(ErrorKind::TcpError)?;
        tcpstream
            .set_read_timeout(Some(Duration::new(2, 0)))
            .map_err(ErrorKind::TcpError)?;
        info!(
            "Writing message to {} ({}):\n{}",
            self.addr, seq_id, &payload
        );
        let bts = tcpstream
            .write(self.mp.encode(&mes, true)?.as_ref())
            .map_err(ErrorKind::TcpError)?;
        info!("Wrote {} bytes ({})", bts, seq_id);
        let mut buf = [0; 256];
        let bts = tcpstream.read(&mut buf).map_err(ErrorKind::TcpError)?;
        info!("Received {} bytes ({})", bts, seq_id);
        if bts == 0 {
            return Err(ErrorKind::BadTcpRead);
        } else {
            debug!(
                "Received response ({}):\n{}",
                seq_id,
                hex::encode(&buf[..bts])
            );
        }
        debug!("Shutting down connection ({})", seq_id);
        tcpstream
            .shutdown(Shutdown::Both)
            .map_err(ErrorKind::TcpError)?;
        self.mp.parse(&buf[..bts])
    }
}
