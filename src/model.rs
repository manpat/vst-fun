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

impl Model {
	pub fn parameter_iter(self) -> impl Iterator<Item=(usize, f32)> {
		let param_map = [
			(2, self.cutoff_dip),
			(3, self.lfo_depth),
			(4, self.filter_lfo_depth),
			(5, self.wonk.0),
			(6, self.wonk.1),
		];

		(0..param_map.len()).map(move |i| param_map[i])
	}

	pub fn diff_with(self, o: Model) -> impl Iterator<Item=(usize, f32)> {
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


