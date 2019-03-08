pub type ParameterMapping = (usize, f32);

// TODO: maybe an enum would be easier
#[derive(Copy, Clone, Debug)]
pub struct Model {
    pub cutoff_dip: f32,
    pub wonk: (f32, f32),
    pub lfo_depth: f32,
    pub filter_lfo_depth: f32,
}

impl Default for Model {
	fn default() -> Model {
		Model {
            cutoff_dip: 100.0,
            wonk: (0.0, 0.0),
            lfo_depth: 5.0,
            filter_lfo_depth: 50.0,
		}
	}
}

pub struct ParameterDef {
	parameter_slot: usize,
	min: f32, max: f32
}

impl ParameterDef {
	const fn new(parameter_slot: usize, min: f32, max: f32) -> Self {
		ParameterDef{ parameter_slot, min, max }
	}
}


impl Model {
	pub const fn parameter_defs() -> [ParameterDef; 5] {
		[
			ParameterDef::new(2, 25.0, 900.0),
			ParameterDef::new(3,  0.0,  20.0),
			ParameterDef::new(4,  0.0, 100.0),
			ParameterDef::new(5, -1.0,   1.0),
			ParameterDef::new(6, -1.0,   1.0),
		]
	}

	pub fn parameter_map(&self) -> [ParameterMapping; 5] {
		[
			(2, self.cutoff_dip),
			(3, self.lfo_depth),
			(4, self.filter_lfo_depth),
			(5, self.wonk.0),
			(6, self.wonk.1),
		]
	}

	pub fn parameter_map_mut(&mut self) -> [(usize, &mut f32); 5] {
		[
			(2, &mut self.cutoff_dip),
			(3, &mut self.lfo_depth),
			(4, &mut self.filter_lfo_depth),
			(5, &mut self.wonk.0),
			(6, &mut self.wonk.1),
		]
	}

	pub fn parameter_iter(self) -> impl Iterator<Item=ParameterMapping> {
		let param_map = self.parameter_map();

		(0..param_map.len()).map(move |i| param_map[i])
	}

	pub fn diff_with(self, o: Model) -> impl Iterator<Item=ParameterMapping> {
		use std::mem::transmute;

		self.parameter_iter()
			.zip(o.parameter_iter())
			.filter_map(|((pos, new), (_, old))| unsafe {
				let new_bytes: u32 = transmute(new);
				let old_bytes: u32 = transmute(old);

				if new_bytes != old_bytes {
					Some((pos, new))
				} else {
					None
				}
			})
	}
}


