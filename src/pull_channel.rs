use std::{
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

pub enum Received<T> {
    Some(T),
    Empty,
    ChannelClosed,
}
/// Returns the most recent received item and discards all the rest.
pub fn pull_until_last<T>(receiver: &Receiver<T>, timeout: Duration) -> Received<T> {
    let mut data = Received::Empty;

    loop {
        match receiver.try_recv() {
            Ok(recv_data) => data = Received::Some(recv_data),
            Err(_) => break,
        }
    }

    match receiver.recv_timeout(timeout) {
        Ok(recv_data) => data = Received::Some(recv_data),
        Err(RecvTimeoutError::Timeout) => {}
        Err(RecvTimeoutError::Disconnected) => match &data {
            Received::Empty => return Received::ChannelClosed,
            _ => (),
        },
    }

    data
}
