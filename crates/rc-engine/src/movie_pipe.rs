//! A bounded decoder mailbox with explicit cancellation and audio-clock demand.
//! No borrowed channel internals, hard-coded offsets, or pointer patching. Closing
//! wakes a producer blocked on a full mailbox; it also prevents wasted colour
//! conversion after a skip. MPEG reference decoding remains in the decoder.
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicU64, Ordering}};
const REALTIME: u64 = 1 << 32;
struct State<T> { queue: VecDeque<T>, closed: bool, sender: bool }
struct Shared<T> { state: Mutex<State<T>>, ready: Condvar, room: Condvar, capacity: usize, demand: AtomicU64 }
pub struct Sender<T>(Arc<Shared<T>>);
pub struct Receiver<T>(Arc<Shared<T>>);
#[derive(Debug, PartialEq, Eq)] pub enum Empty { Pending, Closed }
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    assert!(capacity > 0);
    let shared=Arc::new(Shared {state:Mutex::new(State{queue:VecDeque::with_capacity(capacity),closed:false,sender:true}),ready:Condvar::new(),room:Condvar::new(),capacity,demand:AtomicU64::new(0)});
    (Sender(shared.clone()),Receiver(shared))
}
impl<T> Sender<T> {
    pub fn send(&self, value:T)->Result<(),T> {
        let mut s=self.0.state.lock().unwrap_or_else(|e|e.into_inner());
        while !s.closed && s.queue.len()==self.0.capacity {s=self.0.room.wait(s).unwrap_or_else(|e|e.into_inner());}
        if s.closed {return Err(value)}
        s.queue.push_back(value);self.0.ready.notify_one();Ok(())
    }
    pub fn closed(&self)->bool {self.0.state.lock().unwrap_or_else(|e|e.into_inner()).closed}
    /// Called after `Movie::next_frame`, never instead of MPEG reference decode.
    pub fn should_convert(&self,index:u32)->bool {let d=self.0.demand.load(Ordering::Acquire);!self.closed()&&(d&REALTIME==0||index>=d as u32)}
}
impl<T> Drop for Sender<T> {fn drop(&mut self){let mut s=self.0.state.lock().unwrap_or_else(|e|e.into_inner());s.sender=false;self.0.ready.notify_all();}}
impl<T> Receiver<T> {
    pub fn wanted(&self, index:u32, realtime:bool){self.0.demand.store(index as u64|if realtime{REALTIME}else{0},Ordering::Release);}
    pub fn receive(&self, blocking:bool)->Result<T,Empty> {
        let mut s=self.0.state.lock().unwrap_or_else(|e|e.into_inner());
        loop {
            if s.closed {return Err(Empty::Closed)}
            if let Some(v)=s.queue.pop_front(){self.0.room.notify_one();return Ok(v)}
            if !s.sender {return Err(Empty::Closed)}
            if !blocking {return Err(Empty::Pending)}
            s=self.0.ready.wait(s).unwrap_or_else(|e|e.into_inner());
        }
    }
    pub fn close(&self){let mut s=self.0.state.lock().unwrap_or_else(|e|e.into_inner());s.closed=true;s.queue.clear();self.0.room.notify_all();self.0.ready.notify_all();}
}
impl<T> Drop for Receiver<T> {fn drop(&mut self){self.close();}}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn realtime_skips_only_late_conversion(){let (tx,rx)=channel::<u8>(4);assert!(tx.should_convert(0));rx.wanted(10,true);assert!(!tx.should_convert(9));assert!(tx.should_convert(10));assert!(tx.should_convert(11));rx.wanted(100,false);assert!(tx.should_convert(0));}
    #[test] fn close_unblocks_full_producer(){let (tx,rx)=channel(1);tx.send(1).unwrap();let (done,ready)=std::sync::mpsc::channel();let t=std::thread::spawn(move||done.send(tx.send(2)).unwrap());rx.close();assert_eq!(ready.recv_timeout(std::time::Duration::from_secs(2)).unwrap(),Err(2));t.join().unwrap();}
    #[test] fn receiver_drop_unblocks_producer(){let (tx,rx)=channel(1);tx.send(1).unwrap();drop(rx);assert_eq!(tx.send(2),Err(2));assert!(!tx.should_convert(0));}
    #[test] fn sender_drop_is_end_after_buffer(){let (tx,rx)=channel(2);tx.send(1).unwrap();drop(tx);assert_eq!(rx.receive(true),Ok(1));assert_eq!(rx.receive(true),Err(Empty::Closed));}
    #[test] fn realtime_poll_does_not_block(){let (_tx,rx)=channel::<u8>(1);assert_eq!(rx.receive(false),Err(Empty::Pending));}
    #[test] fn distinct_movies_do_not_share_cancel(){let(a,b)=channel(1);let(c,d)=channel(1);b.close();assert!(a.send(1).is_err());c.send(2).unwrap();assert_eq!(d.receive(false),Ok(2));}
}
