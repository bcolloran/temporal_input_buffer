#[derive(Debug)]
pub struct Ewma {
    exp: f32,
    value: f32,
}

impl Default for Ewma {
    fn default() -> Self {
        Self {
            exp: 0.1,
            value: 0.0,
        }
    }
}

impl Ewma {
    pub fn new(exp: f32) -> Self {
        Self { exp, value: 0.0 }
    }

    pub fn new_with_value(exp: f32, value: f32) -> Self {
        Self { exp, value }
    }

    pub fn with_value(&mut self, value: f32) -> Self {
        Self::new_with_value(self.exp, value)
    }

    pub fn observe(&mut self, x: f32) {
        let a = self.exp;
        self.value = a * x + (1.0 - a) * self.value;
    }

    pub fn set(&mut self, x: f32) {
        self.value = x;
    }

    pub fn set_exp(&mut self, exp: f32) {
        self.exp = exp;
    }

    /// Returns the current value of the EWMA.
    pub fn value(&self) -> f32 {
        self.value
    }
}
