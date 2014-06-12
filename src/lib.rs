#![crate_id = "turbine"]

#![feature(phase)]
#[phase(syntax, link)]
//#![deny(missing_doc)]





extern crate log;
extern crate sync;

use sync::Arc;
use eventprocessor::EventProcessor;
use paddedatomics::Padded64;

mod eventprocessor;
mod waitstrategy;
mod paddedatomics;


struct Turbine {
	finalized: bool,
	epb: Vec<Option<Vec<uint>>>,
	graph: Arc<Vec<Vec<uint>>>,
	cursors: Arc<Vec<Padded64>>
}

impl Turbine {
	pub fn new() -> Turbine {
		let mut epb = Vec::with_capacity(8);
		epb.push(None);	// Root

		Turbine {
			finalized: false,
			epb: epb,
			graph: Arc::new(vec![]),
			cursors: Arc::new(vec![])
		}
	}

	pub fn ep_new(&mut self) -> Result<uint, ()> {
		match self.finalized {
			true => Err(()),
			false => {
					self.epb.push(None);
					Ok(self.epb.len() - 1)
			}
		}
	}

	pub fn ep_depends(&mut self, epb_index: uint, dep: uint) -> Result<(),()> {
		if self.finalized == true {
			return Err(());
		}

		let epb = self.epb.get_mut(epb_index);
		match *epb {
			Some(ref mut v) => v.push(dep),
			None => {
				*epb = Some(vec![dep])
			}
		};
		Ok(())
	}

	pub fn ep_finalize(&mut self, token: uint) -> EventProcessor {
		if self.finalized == false {
			self.finalize_graph();
		}

		EventProcessor::new(self.graph.clone(), self.cursors.clone(), token)
	}

	fn finalize_graph(&mut self) {
		let mut eps: Vec<Vec<uint>> = Vec::with_capacity(self.epb.len());
		let mut cursors: Vec<Padded64> = Vec::with_capacity(self.epb.len() + 1);

		// Add the root cursor
		cursors.push(Padded64::new(-1));

		for node in self.epb.iter().skip(1) {
			let deps: Vec<uint> = match *node {
				Some(ref v) => v.clone(),
				None => vec![0]
			};
			eps.push(deps);
			cursors.push(Padded64::new(-1));
		}

		self.graph = Arc::new(eps);
		self.cursors = Arc::new(cursors);
		drop(&self.epb);
		self.finalized = true;
	}

	pub fn start(&mut self) {
		
	}
}


#[cfg(test)]
mod tests {

	use Turbine;

	#[test]
	fn test_init() {
		let t = Turbine::new();
	}

	#[test]
	fn test_create_epb() {
		let mut t = Turbine::new();
		let e1 = t.ep_new();
	}

	#[test]
	fn test_depends() {
		let mut t = Turbine::new();
		let e1 = t.ep_new().unwrap();
		let e2 = t.ep_new().unwrap();

		t.ep_depends(e2, e1);
	}

	#[test]
	fn test_many_depends() {
		let mut t = Turbine::new();
		let e1 = t.ep_new().unwrap();
		let e2 = t.ep_new().unwrap();
		let e3 = t.ep_new().unwrap();
		let e4 = t.ep_new().unwrap();
		let e5 = t.ep_new().unwrap();
		let e6 = t.ep_new().unwrap();

		/*
			Graph layout:

			e6 --> e1 <-- e2
			       ^      ^
			       |      |
			       +---- e3 <-- e4 <-- e5

		*/
		t.ep_depends(e2, e1);
		t.ep_depends(e5, e4);
		t.ep_depends(e3, e1);
		t.ep_depends(e4, e3);
		t.ep_depends(e3, e2);

		t.ep_finalize(e1);
		t.ep_finalize(e2);
		t.ep_finalize(e3);
		t.ep_finalize(e4);
		t.ep_finalize(e5);
		t.ep_finalize(e6);
	}

	#[test]
	fn test_finalize() {
		let mut t = Turbine::new();
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let event_processor = t.ep_finalize(e1.unwrap());

		let e2 = t.ep_new();
		assert!(e2.is_err() == true);
	}

	#[test]
	fn test_double_finalize() {
		let mut t = Turbine::new();
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let event_processor = t.ep_finalize(e1.unwrap());
		let event_processor2 = t.ep_finalize(e1.unwrap());

		let e2 = t.ep_new();
		assert!(e2.is_err() == true);
	}

	#[test]
	fn test_send_task() {
		let mut t = Turbine::new();
		let e1 = t.ep_new();
		assert!(e1.is_ok() == true);

		let e2 = t.ep_new();
		assert!(e2.is_ok() == true);

		t.ep_depends(e2.unwrap(), e1.unwrap());

		let ep1 = t.ep_finalize(e1.unwrap());
		let ep2 = t.ep_finalize(e2.unwrap());

		spawn(proc() {
			let a = ep1;
		});

		spawn(proc() {
			let b = ep2;
		});
	}

}
