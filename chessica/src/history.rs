use counter::Counter;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct History {
    hash_counter: Counter<u64, u8>
}

impl History {
    pub fn new() -> Self {
        History {
            hash_counter: Counter::new()
        }
    }

    pub fn push(&mut self, hash_value: u64) -> u8 {
        self.hash_counter[&hash_value] += 1;
        self.hash_counter[&hash_value]
    }

    pub fn pop(&mut self, hash_value: u64) {
        self.hash_counter[&hash_value] -= 1;
    }
}
