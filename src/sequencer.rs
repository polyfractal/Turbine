

pub struct Padded64 {
	p: [u64, ..7],
	pubcounter: AtomicuInt
}

impl Padded64 {
	pub fn new(x: uint) -> Padded64 {
		Padded64 {
			p: [0u64,0u64,0u64,0u64,0u64,0u64,0u64],
			counter: AtomicInt::new(x)
		}
	}
}



pub trait Sequencer {
	fn new(buffer_size: uint) -> Self;


	//fn buffer_size(&self) -> uint;
	//fn has_avail_capacity(&self, n: uint) -> bool;
	//fn next(&self, n: uint) -> uint;
	//fn try_next(&self, n:uint) -> Option<uint>;
	//fn remaining_capacity(&self) -> uint;
	//fn claim(&self, sequence: uint);
	//fn publish(&self, sequence: uint);
	//fn publish(&self, low: uint, high: uint);
	//fn is_available(&self, sequence: uint) -> bool;
	//add gaiting sequence
	//remove gaiting
	//newBarrier
	//getMinimumSequence
	//getHighestPublishedSequence
}

struct SingleProducer {
	buffer_size: uint,
	// wait strategy
	cursor: Padded64,
	gaiting_sequences: Vec<Padded64>
}

impl Sequencer for SingleProducer {
	fn new(buffer_size: uint) -> SingleProducer {
		SingleProducer {
			buffer_size: buffer_size,
			cursor: Padded64::new(0),
			gaiting_sequences: Vec::with_capacity(2)
		}
	}


}
