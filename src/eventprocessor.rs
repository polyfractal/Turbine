

use std::sync::Arc;
use waitstrategy::WaitStrategy;
use paddedatomics::Padded64;
use ringbuffer::{RingBuffer, Slot};

/// EventProcessors provide functionality to process and consume data from the ring buffer
pub struct EventProcessor<T> {
    graph: Arc<Vec<Vec<usize>>>,
    cursors: Arc<Vec<Padded64>>,
    token: usize,
    ring: Arc<RingBuffer<T>>
}


impl<T: Slot> EventProcessor<T> {

    /// Instantiate a new EventProcessor.
    ///
    /// This accepts several important parameters and is for internal use only.
    /// - ring: an instance of the ring buffer
    /// - graph: a dependency graph, showing how all the EPs relate to eachother.
    /// - cursors: a vector of Padded64 atomics which act as cursors into the ring buffer
    /// - token: the index in the graph which represents this EP
    pub fn new(ring: Arc<RingBuffer<T>>, graph: Arc<Vec<Vec<usize>>>, cursors: Arc<Vec<Padded64>>, token: usize) -> EventProcessor<T> {
        EventProcessor::<T> {
            graph: graph,
            cursors: cursors,
            token: token,
            ring: ring
        }
    }

    /// Begin waiting for data to arrive from the ring buffer.
    ///
    /// This method is the only "public" method in EventProcessor.rs.
    /// This method accepts a closure as its only parameter.  Once data is received (e.g. all dependencies have been
    /// satisfied and the data is ready to be consumed), this closure is called.  A slice from the ring buffer is passed
    /// to the closure.
    ///
    /// The slice may containe one or more pieces of data to process (this batching adds a lot of performance to Turbine).
    /// The user-code running inside the closure must be capable of handling multiple pieces of data.
    ///
    /// Upon completion of processing the data, the closure must return a Result signaling if it wants the event processor
    /// to continue running, or exit.  A Result of Ok(()) will tell the EP to continue running.  A Result of Err(()) will
    /// shut down the EP.
    ///
    /// ## Example
    ///
    ///```
    ///spawn(proc() {
    ///     event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
    ///         assert!(data.len() == 1);
    ///         assert!(data[0].value == 19);
    ///         return Ok(());
    ///     });
    ///});
    ///```
    pub fn start<F, W: WaitStrategy>(&self, mut f: F)
    where F: FnMut(&[T]) -> Result<(),()> {
        let capacity = self.ring.get_capacity();

        let wait_strategy: W = WaitStrategy::new(capacity);

        let ref dep_eps = self.graph.as_slice()[self.token];
        let mut deps: Vec<&Padded64> = Vec::with_capacity(dep_eps.len());
        for ep in dep_eps.iter() {
            deps.push(&(*self.cursors).as_slice()[*ep]);
        }
        drop(dep_eps);

        let ref cursor = &(*self.cursors).as_slice()[self.token + 1];

        let mask: u64 = capacity as u64 - 1;
        let mut internal_cursor = cursor.load();
        let mut rollover = (false, 0);

        loop {
            debug!("              Current: {}, waiting on: {}", internal_cursor, internal_cursor);

            let available = wait_strategy.wait_for(internal_cursor, &deps);
            debug!("							Available: {}", available);

            let from = (internal_cursor & mask) as usize;
            let mut to = (available & mask) as usize;

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
            } else if to == from {
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

            if rollover.0 {
                debug!("ROlLOVER GET");
                status = unsafe {
                    let data: &[T] = self.ring.get(0, rollover.1);
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
