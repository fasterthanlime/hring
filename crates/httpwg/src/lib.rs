use std::rc::Rc;

use fluke_buffet::{IntoHalves, Piece, PieceList, Roll, RollMut, WriteOwned};
use fluke_h2_parse::{
    nom, BitFlags, Frame, FrameType, IntoPiece, Settings, SettingsFlags, StreamId,
};
use tracing::debug;

pub mod rfc9113;

pub struct Conn<IO: IntoHalves + 'static> {
    w: <IO as IntoHalves>::Write,
    scratch: RollMut,
    pub ev_rx: tokio::sync::mpsc::Receiver<Ev>,
}

pub enum Ev {
    Frame { frame: Frame, payload: Roll },
    IoError { error: std::io::Error },
    Eof,
}

impl<IO: IntoHalves> Conn<IO> {
    pub fn new(io: IO) -> Self {
        let (mut r, w) = io.into_halves();

        let (ev_tx, ev_rx) = tokio::sync::mpsc::channel::<Ev>(1);
        let mut eof = false;
        let recv_fut = async move {
            let mut res_buf = RollMut::alloc()?;
            'read: loop {
                if !eof {
                    res_buf.reserve()?;
                    let res;
                    (res, res_buf) = res_buf.read_into(16384, &mut r).await;
                    let n = res?;
                    if n == 0 {
                        debug!("reached EOF");
                        eof = true;
                    } else {
                        debug!(%n, "read bytes (reading frame header)");
                    }
                }

                if eof && res_buf.is_empty() {
                    break 'read;
                }

                match Frame::parse(res_buf.filled()) {
                    Ok((rest, frame)) => {
                        res_buf.keep(rest);
                        debug!("< {frame:?}");

                        // read frame payload
                        let frame_len = frame.len as usize;
                        res_buf.reserve_at_least(frame_len)?;

                        while res_buf.len() < frame_len {
                            let res;
                            (res, res_buf) = res_buf.read_into(16384, &mut r).await;
                            let n = res?;
                            debug!(%n, len = %res_buf.len(), "read bytes (reading frame payload)");

                            if n == 0 {
                                eof = true;
                                if res_buf.len() < frame_len {
                                    panic!(
                                        "peer frame header, then incomplete payload, then hung up"
                                    )
                                }
                            }
                        }

                        let payload = res_buf.take_at_most(frame_len).unwrap();
                        assert_eq!(payload.len(), frame_len);

                        debug!(%frame_len, "got frame payload");
                        ev_tx.send(Ev::Frame { frame, payload }).await.unwrap();
                    }
                    Err(nom::Err::Incomplete(_)) => {
                        if eof {
                            panic!(
                                "peer sent incomplete frame header then hung up (buf len: {})",
                                res_buf.len()
                            )
                        }

                        continue;
                    }
                    Err(nom::Err::Failure(err) | nom::Err::Error(err)) => {
                        debug!(?err, "got parse error");
                        break;
                    }
                }
            }

            Ok::<_, eyre::Report>(())
        };
        fluke_buffet::spawn(async move { recv_fut.await.unwrap() });

        Self {
            w,
            scratch: RollMut::alloc().unwrap(),
            ev_rx,
        }
    }

    pub async fn write_frame(&mut self, frame: Frame, payload: impl IntoPiece) -> eyre::Result<()> {
        let payload = payload.into_piece(&mut self.scratch)?;
        let frame = frame.with_len(payload.len().try_into().unwrap());

        let header = frame.into_piece(&mut self.scratch)?;
        self.w
            .writev_all_owned(PieceList::single(header).followed_by(payload))
            .await?;
        Ok(())
    }

    pub async fn handshake(&mut self) -> eyre::Result<()> {
        // perform an HTTP/2 handshake as a client

        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        self.w.write_all_owned(&preface[..]).await?;

        self.write_frame(
            Frame::new(
                fluke_h2_parse::FrameType::Settings(Default::default()),
                StreamId::CONNECTION,
            ),
            Settings::default(),
        )
        .await?;

        // now wait for the server's settings frame, which must be the first frame
        match self.ev_rx.recv().await {
            None => {
                panic!("EOF while doing http/2 handshake (we sent preface etc., the peer hung up)");
            }
            Some(ev) => match ev {
                Ev::Frame { frame, payload } => {
                    match frame.frame_type {
                        FrameType::Settings(flags) => {
                            if flags.contains(SettingsFlags::Ack) {
                                panic!("RFC 9113 Section 3.4: server sent a settings frame but it had ACK set")
                            }

                            // good, good! let's acknowledge those
                            self.write_frame(
                                Frame::new(
                                    FrameType::Settings(BitFlags::empty() | SettingsFlags::Ack),
                                    StreamId::CONNECTION,
                                ),
                                payload,
                            )
                            .await?;
                        }
                        _ => {
                            panic!("Expected settings frame, got: {frame:?}")
                        }
                    }
                }
                Ev::IoError { error } => panic!("I/O error during http2 handshake: {error}"),
                Ev::Eof => panic!("Eof during http2 handshake"),
            },
        }

        Ok(())
    }

    pub async fn send(&mut self, buf: impl Into<Piece>) -> eyre::Result<()> {
        self.w.write_all_owned(buf.into()).await?;
        Ok(())
    }
}

pub struct Config {}

pub trait Test<IO: IntoHalves + 'static> {
    fn name(&self) -> &'static str;
    fn run(
        &self,
        config: Rc<Config>,
        conn: Conn<IO>,
    ) -> futures_util::future::LocalBoxFuture<eyre::Result<()>>;
}

#[macro_export]
macro_rules! test_struct {
    ($name: expr, $fn: ident, $struct: ident) => {
        #[derive(Default)]
        pub struct $struct {}

        impl<IO: IntoHalves + 'static> Test<IO> for $struct {
            fn name(&self) -> &'static str {
                $name
            }

            fn run(
                &self,
                config: std::rc::Rc<Config>,
                conn: Conn<IO>,
            ) -> futures_util::future::LocalBoxFuture<eyre::Result<()>> {
                Box::pin($fn(config, conn))
            }
        }
    };
}

#[macro_export]
macro_rules! gen_tests {
    ($body: tt) => {
        #[cfg(test)]
        mod rfc9113 {
            use ::httpwg::rfc9113 as __rfc;

            #[test]
            fn test_3_4() {
                use __rfc::Test3_4 as Test;
                $body
            }

            #[test]
            fn test_4_2() {
                use __rfc::Test4_2 as Test;
                $body
            }
        }
    };
}
