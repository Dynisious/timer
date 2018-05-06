
use std::ops::Deref;
use std::iter::Iterator;
use super::{spawn, Sender, Receiver, TimerSignal,};
pub use super::{mpsc::RecvError, thread::JoinHandle,};

pub struct SubSender {
    subscriber: Sender<TimerSignal>,
    id: u32,
}

impl SubSender {
    pub fn new(subscriber: Sender<TimerSignal>, id: u32) -> Self { Self { subscriber, id } }
    pub fn get_id(&self) -> u32 { self.id }
}

impl Deref for SubSender {
    type Target = Sender<TimerSignal>;

    fn deref(&self) -> &Self::Target { &self.subscriber }
}

pub struct TimerSub {
    subscription: Receiver<TimerSignal>,
    id: u32,
}

impl TimerSub {
    pub fn new(subscription: Receiver<TimerSignal>, id: u32) -> Self { Self { subscription, id } }
    pub fn get_id(&self) -> u32 { self.id }
    pub fn signal(&self) -> Option<TimerSignal> { self.subscription.recv().ok() }
    pub fn run<F, P>(self, mut run: F) -> JoinHandle<()>
        where F: 'static + FnMut(P) + Send + Sync,
            P: From<TimerSignal>, {
        spawn(move || while {
            if let Some(signal) = self.signal() {
                run(signal.into());

                match signal {
                    TimerSignal::Subscribed
                    | TimerSignal::Tick
                    | TimerSignal::TimerStopped => true,
                    TimerSignal::Unsubscribed => false,
                }
            } else { false }
        } {})
    }
}

impl Iterator for TimerSub {
    type Item = TimerSignal;

    fn next(&mut self) -> Option<Self::Item> { self.signal() }
}
