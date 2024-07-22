pub enum BackoffVariant {
    Exponential,
}

impl BackoffVariant {
    fn get_multiplier(&self, tries: u64) -> u64 {
        return match self {
            Self::Exponential => tries.pow(2),
        };
    }
}

pub struct Backoff {
    variant: BackoffVariant,
    tries: u64,
    max: Option<u64>,
}

impl Backoff {
    pub fn new(variant: BackoffVariant, max: Option<u64>) -> Self {
        Backoff {
            variant,
            tries: 0,
            max,
        }
    }

    pub fn reset(&mut self) {
        self.tries = 0;
    }

    pub fn next(&mut self) -> u64 {
        let mut val = self.variant.get_multiplier(self.tries);
        if let Some(max) = self.max {
            val = std::cmp::min(val, max);
        }
        self.tries += 1;
        val
    }
}
