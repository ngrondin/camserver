use std::{collections::HashMap, net::UdpSocket, sync::{Arc}, thread};

/*use actix_web::{http::Error, web::Bytes};
use futures_core::Stream;

use crate::utils::Timer;*/


pub trait StreamReceiverState {
    fn set_stream_image(&self, stream_id: u8, data: Arc<Vec<u8>>);
}

#[derive(Clone)]
struct StreamImage {
    id: u8,
    bytes: Vec<u8>
}

#[derive(Clone)]
pub struct StreamReceiver<T: StreamReceiverState + Clone> {
    state: Arc<T>,
    images: HashMap<u8, StreamImage>
}


impl<T: StreamReceiverState + 'static + Sync + Send + Clone> StreamReceiver<T> {
    pub fn init(state: T) {
        spawn_stream_receiver_thread(state);
    }

    pub fn recv_bytes(&mut self, stream_id: u8, img_id: u8, packet_id: u16, data: &[u8]) {
        if !self.images.contains_key(&stream_id) {
            self.images.insert(stream_id, StreamImage { id: img_id, bytes: vec![] });
        }
        let image = self.images.get_mut(&stream_id).unwrap();
        if image.id != img_id {
            self.state.set_stream_image(stream_id, Arc::new(image.bytes.clone()));
            image.id = img_id;
            image.bytes.clear();
        }
        let start = (packet_id * 500) as usize;
        let end = start + data.len();
        if image.bytes.len() < end {
            image.bytes.resize(end, 0);
        }
        for i in 0..data.len() {
            image.bytes[start + i] = data[i];
        }
    }
}

fn spawn_stream_receiver_thread<T: StreamReceiverState + 'static + Sync + Send + Clone>(state: T) {
    let mut streamer = StreamReceiver{ images: HashMap::new(), state: Arc::new(state) };
    thread::spawn(move || {
        let socket = UdpSocket::bind("0.0.0.0:10999").unwrap();
        let mut buf = [0; 520];
        loop {
            let (len, _src) = socket.recv_from(&mut buf).unwrap();
            let stream_id = buf[0];
            let img_id = buf[1];
            let packet_id = ((buf[2] as u16) << 8) | (buf[3] as u16);
            let data = &buf[4..(4+len)];
            streamer.recv_bytes(stream_id, img_id, packet_id, data);
        }
    });
}

/*
const HTTP_CONTENT_TYPE: &'static str = "multipart/x-mixed-replace;boundary=123456789000000000000987654321";
const HTTP_PART_BOUNDARY: &'static str = "\r\n--123456789000000000000987654321\r\n";
const HTTP_PART_CONTENT_TYPE: &'static str = "Content-Type: image/jpeg\r\n";

pub struct JPGStreamSender {
    pub rx: Receiver<Arc<Vec<u8>>>,
    pub content_type: &'static str,
    pub since_last: Timer
}

impl JPGStreamSender {
    pub fn new(rx: Receiver<Arc<Vec<u8>>>) -> Self {
        JPGStreamSender { rx, content_type: HTTP_CONTENT_TYPE, since_last: Timer::new() }
    }
}

impl Stream for JPGStreamSender {
    type Item = Result<Bytes, Error>;

    fn poll_next(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let _sl = self.since_last.mark();
        let _timer_recv = Timer::new();
        match self.rx.recv() {
            Ok(data) => {
                let http_len = &format!("Content-Length: {}\r\n\r\n", data.as_ref().len())[..];
                let data_slice = &data.as_ref()[..];
                let d = [HTTP_PART_BOUNDARY.as_bytes(), HTTP_PART_CONTENT_TYPE.as_bytes(), http_len.as_bytes(), data_slice].concat();
                let bytes = Bytes::from(d);
                //println!("Sending image: {}ms to receive, {}ms since last", timer_recv.mark(), sl);
                self.get_mut().since_last.reset();
                Poll::Ready(Some(Ok(bytes)))
            },
            Err(_) => {
                Poll::Pending
            },
        }
    }
}
    */