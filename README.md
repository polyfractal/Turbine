## Turbine

Turbine is a high-performance, non-locking, inter-task communication library written in Rust.

### Overview
Turbine is a spiritual port of the [LMAX-Disruptor pattern](https://github.com/LMAX-Exchange/disruptor).  Although the abstractions used in this library are different from those in the original Disruptor, they share similar concepts and operate on the same principle

Turbine is essentially a channel on steroids, permitting data passing and communication between tasks in a very efficient manner.  Turbine uses a variety of techniques -- such as non-locking ring buffer, single producer, consumer dependency management and batching -- to produce very low latencies and high throughput.

So why would you choose Turbine?  Turbine is excellent if it forms the core of your application.  Turbine, like Disruptor, is used if several consumers need act on the data in parallel, and then allow the "business" logic to execute.
Turbine is useful when you need to process millions of events per second.

On simple, synthetic tests, Turbine exceeds 30 million messages per second between tasks, while channels cap out around 4m (on the test hardware).

That said, Turbine does not replace channels for a variety of reasons.

- Channels are much simpler to use
- Channels are more efficient if you have low or inconsistent communication requirements
- Channels can be MPSC (multi-producer, single-consumer) while Turbine is SPMC
- Turbine requires significant memory overhead to initialize (the ring buffer)

### Usage

```rust
// Initialize a new Turbine
let mut turbine: Turbine<TestSlot> = Turbine::new(1024);

// Create an EventProcessorBulder
let epBuilder = match turbine.ep_new() {
    Ok(ep) => ep,
	Err(_) => fail!("Failed to create new EventProcessor!")
};

// Finalize and retrieve an EventProcessor
let event_processor = turbine.ep_finalize(epBuilder);

// Spawn a new thread, wait for data to arrive
spawn(proc() {
	event_processor.start::<BusyWait>(|data: &[TestSlot]| -> Result<(),()> {
	    // ... process work here ... //
	});
});

// Write data into Turbine
let mut x: TestSlot = Slot::new();
x.value = 19;
turbine.write(x);
```

### Performance
Turbine has not been tuned or optimized yet, and there are still a lot of ugly debug lines laying around.  That said, it's already pretty darn fast.

On my Macbook Air, Turbine sustains ~30m messages per second between threads (passing simple integers).  In comparison, channels max out around 4-7m messages per second.

Perhaps more interesting is latency.  Below is a log-log plot of latency (in nanoseconds) for Turbine and channels.  Latency was measured by sending a single event and pausing for 10 microseconds, which helps assure that neither communication method is saturated with events.

![](turbine1.png)

As you can see, Turbine averages around 250ns per message, while channels average around 16,000ns (16 microseconds).  Because log-log plots are sometimes hard to interpret, here is a log-linear plot.  THe x-axis is still logarithmic, but the latency on the y-axis is linear:

![](turbine2.png)

As you can see, there is a rather large difference between the two.

There is definitely tuning left to be done.  The theoretical minimum latency on my test hardware is ~40ns, based on the latency of inter-core communication.  Which means the current performance is about 4x slower than it could be...plenty of tuning to do!
