

use sync::Arc;
use waitstrategy::{WaitStrategy};
use paddedatomics::Padded64;
use ringbuffer::{RingBuffer, Slot};


pub struct EventProcessor<T> {
    graph: Arc<Vec<Vec<uint>>>,
    cursors: Arc<Vec<Padded64>>,
    token: uint,
    ring: Arc<RingBuffer<T>>
}


impl<T: Slot> EventProcessor<T> {
    pub fn new(ring: Arc<RingBuffer<T>>, graph: Arc<Vec<Vec<uint>>>, cursors: Arc<Vec<Padded64>>, token: uint) -> EventProcessor<T> {
        EventProcessor::<T> {
            graph: graph,
            cursors: cursors,
            token: token,
            ring: ring
        }
    }


    pub fn start<W: WaitStrategy>(&self, f: |data: &[T]| -> Result<(),()>) {
        let capacity = self.ring.get_capacity();

        let wait_strategy: W = WaitStrategy::new(capacity);

        let dep_eps = self.graph.get(self.token);
        let mut deps: Vec<&Padded64> = Vec::with_capacity(dep_eps.len());
        for ep in dep_eps.iter() {
            deps.push((*self.cursors).get(*ep));
        }
        drop(dep_eps);

        let cursor = (*self.cursors).get(self.token + 1);

        let mask: u64 = capacity as u64 - 1;
        let mut internal_cursor = cursor.load();
        let mut rollover = (false, 0);

        loop {
            debug!("              Current: {}, waiting on: {}", internal_cursor, internal_cursor);

            let available = wait_strategy.wait_for(internal_cursor, &deps);
            debug!("							Available: {}", available);

            let from = (internal_cursor & mask) as uint;
            let mut to = (available & mask) as uint;

            debug!("              from: {}, to: {} -- {}", from, to, (to < from));
            if to < from {
                debug!("						ROLLOVER");
                rollover = (true, to);
                to = capacity;
            } else if (to == from) && (internal_cursor < available) {
                //complete buffer request
                debug!("						ROLLOVER (total) -- ({} == {}) && ({} < {})", to, from, internal_cursor, available);
                rollover = (true, to);
                to = capacity;
            } else if (to == from) {
                debug!("						WTF to == from    -- ({} == {}) && ({} < {})", to, from, internal_cursor, available);
            }


            debug!("              Post-modification from: {}, to: {} -- {}", from, to, (to < from));

            // This is safe because the Producer task cannot invalidate these slots
            // before we increment our cursor.  Since the slice is borrowed out, we
            // know it will be returned after the function call ends.  The slice will
            // be dropped after the unsafe block, and *then* we increment our cursor
            let mut status = unsafe {
                let data: &[T] = self.ring.get(from, to);
                f(data)
            };

            if rollover.val0() == true {
                debug!("ROlLOVER GET");
                status = unsafe {
                    let data: &[T] = self.ring.get(0, rollover.val1());
                    f(data)
                };
                rollover = (false,0);
            }

            internal_cursor = available;
            cursor.store(internal_cursor);
            debug!("					Finished processing event.  Cursor @ {} ({})", available, available & mask);

            match status {
                Err(_) => break,
                Ok(_) => {}
            };

        }
        debug!("BusyWait::end");
    }
}
