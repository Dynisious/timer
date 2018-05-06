
use std::thread::{self, JoinHandle,};
pub use std::time::Duration;
use std::sync::{mpsc::channel, Arc, Mutex,};
use std::thread::spawn;
use std::sync::mpsc::{self, Sender, Receiver,};

mod subscription;

use self::subscription::*;
pub use self::subscription::TimerSub;

pub struct Timer {
    tick: Duration,
    subscribers: Arc<Mutex<Vec<SubSender>>>,
    timer: Option<(JoinHandle<()>, Sender<TimerMessage>)>,
    next_sub_id: u32,
}

impl Timer {
    pub fn new(tick: Duration) -> Timer {
        Self {
            tick,
            subscribers: Arc::new(Mutex::new(Vec::default())),
            timer: None,
            next_sub_id: 1,
        }
    }
    pub fn start(mut self) -> Self {
        let (sender, receiver) = channel();

        if let None = self.timer {
            let tick = self.tick;
            let subscribers = self.subscribers.clone();

            self.timer = Some((
                spawn(move || Self::run_timer(receiver, subscribers, tick)),
                sender
            ))
        }

        self
    }
    pub fn stop(mut self) -> Self {
        if let Some((thread, input)) = self.timer.take() {
            input.send(TimerMessage::Stop).ok();
            thread.join().ok();
        }

        self
    }
    pub fn subscribe(&mut self) -> Option<TimerSub> {
        if let Some(mut subscribers) = self.subscribers.lock().ok()
            .filter(|subscribers| subscribers.iter()
                .find(|active| active.get_id() == self.next_sub_id).is_none()
            ) {
            let (sender, sub) = channel();
            let sender = SubSender::new(sender, self.next_sub_id);
            let sub = TimerSub::new(sub, self.next_sub_id);
            
            self.next_sub_id += 1;
            sender.send(TimerSignal::Subscribed).ok();
            subscribers.push(sender);
            Some(sub)
        } else { None }
    }
    pub fn unsubscribe(&mut self, sub: TimerSub) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            if let Some(index) = subscribers.iter().enumerate()
                .find(|(_, active)| active.get_id() == sub.get_id())
                .map(|(index, _)| index) {
                subscribers.swap_remove(index)
                    .send(TimerSignal::Unsubscribed).ok();
            }
        }
    }
    fn run_timer(input: Receiver<TimerMessage>, subscribers: Arc<Mutex<Vec<SubSender>>>, tick: Duration) {
        use std::thread::sleep;
        use std::time::Instant;

        let mut prev = Instant::now();

        'TimerRunning: loop {
            let mut now;
            let mut difference;

            for msg in input.try_iter() {
                match msg {
                    TimerMessage::Stop => break 'TimerRunning,
                }
            }

            while {
                now = Instant::now();
                difference = now - prev;

                difference < tick
            } { sleep(tick) }
            prev = now;

            if let Ok(mut subscribers) = subscribers.lock() {
                if !subscribers.is_empty() {
                    while difference >= tick {
                        let mut index = 0;

                        while index < subscribers.len() {
                            if let Err(_) = subscribers[index].send(TimerSignal::Tick) {
                                subscribers.swap_remove(index);
                            } else {
                                index += 1;
                            }
                        }

                        difference -= tick;
                    }
                }
            }
        }

        if let Ok(subscribers) = subscribers.lock() {
            for sub in subscribers.iter() {
                sub.send(TimerSignal::TimerStopped).ok();
            }
        }
    }
}

#[derive(Clone, Copy)]
enum TimerMessage {
    Stop,
}

#[derive(Clone, Copy)]
pub enum TimerSignal {
    Tick,
    Subscribed,
    Unsubscribed,
    TimerStopped,
}
