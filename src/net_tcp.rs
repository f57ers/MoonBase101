use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::TcpStream;

const MAX_FRAME: usize = 256 * 1024;
const MAX_QUEUE: usize = 2 * 1024 * 1024;

pub struct Connection {
    pub stream: TcpStream,
    pub peer_id: Option<u32>,
    input: Vec<u8>,
    output: VecDeque<Vec<u8>>,
    written: usize,
    queued: usize,
    failed: bool,
}

impl Connection {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            peer_id: None,
            input: Vec::new(),
            output: VecDeque::new(),
            written: 0,
            queued: 0,
            failed: false,
        })
    }
    pub fn queue(&mut self, payload: &[u8]) -> io::Result<()> {
        if payload.len() > MAX_FRAME || self.queued + payload.len() + 4 > MAX_QUEUE {
            self.failed = true;
            return Err(io::ErrorKind::InvalidData.into());
        }
        let mut frame = Vec::with_capacity(payload.len() + 4);
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload);
        self.queued += frame.len();
        self.output.push_back(frame);
        Ok(())
    }
    pub fn pump(&mut self) -> io::Result<Vec<Vec<u8>>> {
        if self.failed {
            return Err(io::ErrorKind::ConnectionAborted.into());
        }
        let mut budget = 128 * 1024;
        while let Some(frame) = self.output.front() {
            if budget == 0 {
                break;
            }
            let end = (self.written + budget).min(frame.len());
            match self.stream.write(&frame[self.written..end]) {
                Ok(0) => return Err(io::ErrorKind::ConnectionAborted.into()),
                Ok(n) => {
                    self.written += n;
                    self.queued -= n;
                    budget -= n;
                    if self.written == frame.len() {
                        self.output.pop_front();
                        self.written = 0;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        let mut buf = [0; 8192];
        for _ in 0..16 {
            match self.stream.read(&mut buf) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(n) => self.input.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        decode_frames(&mut self.input)
    }
}

fn decode_frames(input: &mut Vec<u8>) -> io::Result<Vec<Vec<u8>>> {
    let mut frames = Vec::new();
    let mut offset = 0;
    while input.len() - offset >= 4 {
        let size = u32::from_be_bytes(input[offset..offset + 4].try_into().unwrap()) as usize;
        if size == 0 || size > MAX_FRAME {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if input.len() - offset < size + 4 {
            break;
        }
        frames.push(input[offset + 4..offset + 4 + size].to_vec());
        offset += 4 + size;
    }
    input.drain(..offset);
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_and_coalesced_frames_are_preserved() {
        let mut bytes = vec![0, 0];
        assert!(decode_frames(&mut bytes).unwrap().is_empty());
        bytes.extend_from_slice(&[0, 3, 1]);
        assert!(decode_frames(&mut bytes).unwrap().is_empty());
        bytes.extend_from_slice(&[2, 3, 0, 0, 0, 2, 4, 5]);
        assert_eq!(
            decode_frames(&mut bytes).unwrap(),
            vec![vec![1, 2, 3], vec![4, 5]]
        );
        assert!(bytes.is_empty());
        bytes.extend_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode_frames(&mut bytes).is_err());
    }
}
