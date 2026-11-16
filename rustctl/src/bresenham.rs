pub(crate) struct MultiAxisPlanner<const N: usize> {
    counts: [i64; N],
    accum: [i64; N],
    max_steps: i64,
    remaining: i64,
}

impl<const N: usize> MultiAxisPlanner<N> {
    pub(crate) fn new(steps: [i64; N]) -> Self {
        let counts: [i64; N] = std::array::from_fn(|i| steps[i].abs());
        let max_steps = counts.iter().copied().max().unwrap_or(0);
        Self {
            counts,
            accum: [0; N],
            max_steps,
            remaining: max_steps,
        }
    }
}

impl<const N: usize> Iterator for MultiAxisPlanner<N> {
    type Item = [bool; N];

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining <= 0 {
            return None;
        }
        self.remaining -= 1;

        let mut pulses = [false; N];
        pulses
            .iter_mut()
            .zip(self.accum.iter_mut())
            .zip(self.counts.iter())
            .for_each(|((pulse, accum), &count)| {
                *accum += count;
                if *accum >= self.max_steps {
                    *pulse = true;
                    *accum -= self.max_steps;
                }
                debug_assert!(*accum >= 0 && *accum < self.max_steps.max(1));
            });
        Some(pulses)
    }
}
