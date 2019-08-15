mod prelude;
pub use crate::prelude::*;

pub const RANGED_MELEE_MULT: f64 = 0.25; //when ranged unit is being attacked by a melee, their damage goes down significantly
pub const END_FIGHT_HEAL_AMOUNT: f64 = 0.2; //after a fight, all surviving units heal for this fraction of their max hp

#[derive(Debug,Copy,Clone,Serialize,Deserialize)]
pub struct UnitView {
	pub class: Option<Class>,
	pub element: Option<Element>,
	pub frac_hp: Option<f64>,
}

impl UnitView {
	pub fn new() -> Self {
		Self {
			class: None,
			element: None,
			frac_hp: None
		}
	}
	
	pub fn hp(&self) -> f64 {
		self.class.map(|x| x.base_hp()).unwrap_or(1.0) * self.element.map(|x| x.hp_mult()).unwrap_or(1.0) * self.frac_hp.unwrap_or(1.0)
	}
	
	pub fn max_hp(&self) -> f64 {
		self.class.map(|x| x.base_hp()).unwrap_or(1.0) * self.element.map(|x| x.hp_mult()).unwrap_or(1.0)
	}
	
	pub fn damage(&self) -> f64 {
		self.class.map(|x| x.base_damage()).unwrap_or(1.0) * self.element.map(|x| x.hp_mult()).unwrap_or(1.0)
	}
}

#[derive(Debug,Copy,Clone,Eq,PartialEq,Serialize,Deserialize)]
pub enum Class {
	Melee,
	Ranged,
}

impl Class {
	pub fn new() -> Self {
		match random() % 2 {
			0 => Class::Melee,
			1 => Class::Ranged,
			_ => unreachable!(),
		}
	}
	
	pub fn base_hp(&self) -> f64 {
		match self {
			Class::Melee => 0.7,
			Class::Ranged => 1.0,
		}
	}
	
	pub fn base_damage(&self) -> f64 {
		match self {
			Class::Melee => 1.0,
			Class::Ranged => 1.45,
		}
	}
	
	pub fn base_block(&self) -> f64 {
		match self {
			Class::Melee => 0.05,
			Class::Ranged => 0.0,
		}
	}
	
	pub fn base_regen(&self) -> f64 {
		match self {
			Class::Melee => 0.3,
			Class::Ranged => 0.2,
		}
	}
}

#[derive(Debug,Copy,Clone,Eq,PartialEq,Serialize,Deserialize)]
pub enum Element {
	Red,
	Green,
	Blue,
}

impl Element {
	pub fn new() -> Self {
		match random() % 3 {
			0 => Element::Red,
			1 => Element::Green,
			2 => Element::Blue,
			_ => unreachable!(),
		}
	}
	
	pub fn hp_mult(&self) -> f64 {
		match self {
			Element::Red => 5.0 / 6.0,
			Element::Green => 6.0 / 5.0,
			Element::Blue => 1.0,
		}
	}
	
	pub fn damage_mult(&self) -> f64 {
		match self {
			Element::Red => 6.0 / 5.0,
			Element::Green => 5.0 / 6.0,
			Element::Blue => 1.0,
		}
	}
	
	pub fn damage_vs(&self, other: &Self) -> f64 {
		match self {
			Element::Red => match other {
				Element::Red => 1.0,
				Element::Green => 3.0 / 2.0,
				Element::Blue => 2.0 / 3.0,
			},
			Element::Green => match other {
				Element::Red => 2.0 / 3.0,
				Element::Green => 1.0,
				Element::Blue => 3.0 / 2.0,
			},
			Element::Blue => match other {
				Element::Red => 3.0 / 2.0,
				Element::Green => 2.0 / 3.0,
				Element::Blue => 1.0,
			},
		}
	}
}

#[derive(Debug,Copy,Clone,Serialize,Deserialize)]
pub struct AuthInfo {
	pub id: [u8; 32],
	pub data: [u64; 4],
}

use std::io::Read;
use serde::de::DeserializeOwned;

#[macro_export]
macro_rules! l {
	() => { &concat!(file!(), " ", line!()) }
}

struct DebugStream<R>(R, Vec<u8>);

impl<R: Read> Read for DebugStream<R> {
	fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize> {
		let r = self.0.read(buf);
		self.1 = buf.as_ref().into();
		r
	}
}


pub fn try_recv<T: DeserializeOwned, R: Read>(stream: R, limit: Option<u64>) -> Result<Option<T>, Error> {
	let mut cfg = bincode::config();
	if let Some(limit) = limit {
		cfg.limit(limit);
	}
	use bincode::ErrorKind::*;
	
	let mut stream = DebugStream(stream, Vec::new());
	
	match cfg.deserialize_from(&mut stream) {
		Ok(result) => {
			Ok(Some(result))
		},
		Err(e) => match *e {
			Io(e) => {
				match e.kind() {
					std::io::ErrorKind::WouldBlock => {
						Ok(None)
					},
					_ => Err(Box::new(e)),
				}
			},
			_ => {
				println!("error in try_recv, binary: {:?}",stream.1);
				Err(e)
			},
		}
	}
}

#[derive(Debug,Copy,Clone,Serialize,Deserialize)]
pub struct Unit {
	pub class: Class,
	pub element: Element,
	pub hp: f64,
	pub max_hp: f64,
}

#[derive(Debug,Copy,Clone,Serialize,Deserialize)]
pub struct MoveOption {
	pub id: u64,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct FightRecording {
	pub won: bool,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum ServerPacket { //packet from server
	Team(Vec<Unit>),
	Opponent(bool, Vec<UnitView>),
	MoveOptions(Vec<MoveOption>),
	Message(String),
	Fight(FightRecording),
}

#[derive(Debug,Copy,Clone,Serialize,Deserialize)]
pub enum ClientPacket { //packet from client
	Move(u64),
	Fight(bool),
	Rearrange(usize, usize),
	Disconnect,
}

pub fn serialize_small_string(s: &String) -> Result<[u8; 32], ()> {
	let s = serialize(s).unwrap();
	for i in 1..8 {
		debug_assert!(s[i] == 0);
	}
	if s.len() < 40 {
		let mut r = [0; 32];
		r[0] = s[0];
		let mut i = 0;
		for &s in s.iter().skip(8) {
			i += 1;
			r[i] = s;
		}
		Ok(r)
	} else {
		Err(())
	}
}

pub fn deserialize_small_string(v: &[u8]) -> Result<String, Box<bincode::ErrorKind>> {
	let mut bytes = vec!(v[0]);
	(0..7).map(|_| bytes.push(0)).last();
	for &v in v.iter().skip(1) {
		bytes.push(v);
	}
	deserialize(&bytes)
}
