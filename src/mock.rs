use bytes::{Buf, BufMut, Bytes, BytesMut};
use http_body::Body;
use prost::Message;
use std::{
    collections::VecDeque,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tokio::sync::mpsc::Receiver;

use tonic::{
    Status,
    codec::{DecodeBuf, Decoder},
};

// Internal state for channel-based MockBody
struct ChannelState<T> {
    receiver: Receiver<T>,
    buffer: VecDeque<Bytes>,
    waker: Option<Waker>,
    closed: bool,
}

#[derive(Clone)]
enum MockBodySource<T> {
    // Static data from a Vec
    Static(VecDeque<Bytes>),
    // Dynamic data from a channel
    Channel(Arc<Mutex<ChannelState<T>>>),
}

#[derive(Clone)]
pub struct MockBody<T = Box<dyn Message>> {
    source: MockBodySource<T>,
}

impl<T: Message + Send + 'static> MockBody<T> {
    pub fn new(data: Vec<impl Message>) -> Self {
        let mut queue: VecDeque<Bytes> = VecDeque::with_capacity(16);
        for msg in data {
            let buf = Self::encode(msg);
            queue.push_back(buf);
        }

        MockBody {
            source: MockBodySource::Static(queue),
        }
    }

    /// Create a MockBody from a channel receiver
    ///
    /// This allows for dynamic streaming of messages without collecting them all upfront.
    pub fn from_channel(receiver: Receiver<T>) -> Self {
        let state = ChannelState {
            receiver,
            buffer: VecDeque::new(),
            waker: None,
            closed: false,
        };

        MockBody {
            source: MockBodySource::Channel(Arc::new(Mutex::new(state))),
        }
    }

    pub fn len(&self) -> usize {
        match &self.source {
            MockBodySource::Static(queue) => queue.len(),
            MockBodySource::Channel(state) => {
                let state = state.lock().unwrap();
                state.buffer.len()
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // see: https://github.com/hyperium/tonic/blob/1b03ece2a81cb7e8b1922b3c3c1f496bd402d76c/tonic/src/codec/encode.rs#L52
    fn encode(msg: impl Message) -> Bytes {
        let mut buf = BytesMut::with_capacity(256);

        buf.reserve(5);
        unsafe {
            buf.advance_mut(5);
        }
        msg.encode(&mut buf).unwrap();
        {
            let len = buf.len() - 5;
            let mut buf = &mut buf[..5];
            buf.put_u8(0); // byte must be 0, reserve doesn't auto-zero
            buf.put_u32(len as u32);
        }
        buf.freeze()
    }
}

impl<T: Message + Send + 'static> Body for MockBody<T> {
    type Data = Bytes;
    type Error = Status;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();

        match &mut this.source {
            MockBodySource::Static(queue) => {
                // Return data from the static queue
                if let Some(data) = queue.pop_front() {
                    Poll::Ready(Some(Ok(http_body::Frame::data(data))))
                } else {
                    Poll::Ready(None)
                }
            }
            MockBodySource::Channel(state_arc) => {
                let mut state = state_arc.lock().unwrap();

                // If we have buffered data, return it
                if let Some(data) = state.buffer.pop_front() {
                    return Poll::Ready(Some(Ok(http_body::Frame::data(data))));
                }

                // If the channel is closed and we have no more buffered data, we're done
                if state.closed {
                    return Poll::Ready(None);
                }

                // Try to receive a message from the channel
                match state.receiver.try_recv() {
                    Ok(msg) => {
                        // Got a message, encode it and return
                        let buf = Self::encode(msg);
                        Poll::Ready(Some(Ok(http_body::Frame::data(buf))))
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        // Channel is empty but not closed, register waker and return Pending
                        state.waker = Some(cx.waker().clone());
                        Poll::Pending
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        // Channel is closed, mark as closed and return None
                        state.closed = true;
                        Poll::Ready(None)
                    }
                }
            }
        }
    }
}

/// A [`Decoder`] that knows how to decode `U`.
#[derive(Debug, Clone, Default)]
pub struct ProstDecoder<U>(PhantomData<U>);

impl<U> ProstDecoder<U> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<U: Message + Default> Decoder for ProstDecoder<U> {
    type Item = U;
    type Error = Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let item = Message::decode(buf.chunk())
            .map(Option::Some)
            .map_err(|e| Status::internal(e.to_string()))?;

        buf.advance(buf.chunk().len());
        Ok(item)
    }
}
