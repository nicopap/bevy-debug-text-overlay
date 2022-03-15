//! Keep track of free/unfree space on a 1d line
//!
//! This is very generic because oops! Too much fun!
//!
//! A [`Block`] represents something take takes [`Block::size`] space or a gap
//! in space.
//!
//! [`Blocks`] acts like a heap, where you can add and remove things.
use std::iter::{once, Sum};
use std::ops::{AddAssign, Sub};

/// A quantity that can be added, substracted and has a ZERO. Generally known
/// as a monoid.
pub(crate) trait Summable:
    Sum + for<'a> AddAssign<&'a Self> + Sub<Output = Self> + PartialOrd + PartialEq + Copy
{
    const ZERO: Self;
}
impl Summable for f32 {
    const ZERO: Self = 0.0;
}

/// A `Block` represents something take takes [`Block::size`] space or a gap
/// in space `S`. Each occupied `Block` is identified with `Id`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Block<Id, S> {
    /// A void of size `S` left from something that was removed.
    Gap(S),
    /// Something identified by `Id` that takes space `S`.
    Full(Id, S),
}
impl<Id, S> Block<Id, S> {
    fn size(&self) -> S
    where
        S: Copy,
    {
        match self {
            Block::Full(_, size) => *size,
            Block::Gap(size) => *size,
        }
    }
    fn has_id(&self, id: &Id) -> bool
    where
        Id: PartialEq,
    {
        matches!(self, Block::Full(self_id, _) if self_id == id)
    }
}
/// `Blocks` manage resource allocation on a 1D line of generic type
/// `S: `[`Summable`], and each allocation block is identified by `Id`.
#[derive(Debug)]
pub(crate) struct Blocks<Id, S>(Vec<Block<Id, S>>);
impl<Id, S> Default for Blocks<Id, S> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
struct Gap<S> {
    index: usize,
    gap_size: S,
}
impl<Id, S> Blocks<Id, S>
where
    Id: PartialEq,
    S: Summable,
{
    /// This assumes, `Self` is [cleaned up](Blocks::cleanup).
    fn first_gap_of_size(&self, size: S) -> Option<Gap<S>> {
        self.0
            .iter()
            .enumerate()
            .find(|(_, block)| matches!(block, Block::Gap(gap) if gap >= &size))
            .map(|(index, block)| Gap { index, gap_size: block.size() })
    }
    fn replace_gap(&mut self, gap: Option<&Gap<S>>, id: Id, size: S) {
        let to_insert = Block::Full(id, size);
        match gap {
            Some(Gap { index, gap_size }) if gap_size > &size => {
                let gap = Block::Gap(*gap_size - size);
                self.0.splice(index..=index, [to_insert, gap].into_iter());
                self.cleanup();
            }
            Some(Gap { index, .. }) => self.0[*index] = to_insert,
            None => self.0.push(to_insert),
        };
    }
    pub(crate) fn insert_size(&mut self, id: Id, size: S) -> S {
        let gap_range = self.first_gap_of_size(size);
        let old_len = self.0.len();
        self.replace_gap(gap_range.as_ref(), id, size);
        let start = gap_range.map_or(old_len, |Gap { index, .. }| index);
        self.0.iter().take(start).map(Block::size).sum()
    }
    pub(crate) fn remove(&mut self, id: Id) {
        if let Some(to_remove) = self.0.iter_mut().find(|block| block.has_id(&id)) {
            *to_remove = Block::Gap(to_remove.size());
        }
        self.cleanup();
    }
    /// Remove [`Block::Gap`] at the end of `self` and merges adjacent gaps.
    fn cleanup(&mut self) {
        let mut cur_gap = S::ZERO;
        let mut gap_start = 0;
        let mut splice_commands = Vec::new();
        for (i, block) in self.0.iter().enumerate() {
            match block {
                Block::Gap(gap) if cur_gap == S::ZERO => {
                    gap_start = i;
                    cur_gap += gap;
                }
                Block::Gap(gap) => cur_gap += gap,
                // There is multiple adjacent gaps
                Block::Full(..) if cur_gap != S::ZERO && i - gap_start > 1 => {
                    splice_commands.push((gap_start, i, cur_gap));
                    cur_gap = S::ZERO;
                }
                Block::Full(..) => cur_gap = S::ZERO,
            }
        }
        for (start, end, size) in splice_commands.into_iter() {
            let to_insert = Block::Gap(size);
            self.0.splice(start..end, once(to_insert));
        }
        if matches!(self.0.last(), Some(Block::Gap(_))) {
            self.0.pop().expect("We just tested Vec::last is Some");
        }
    }
}
#[cfg(test)]
mod tests {
    // TODO: very small deltas on S==f32 may cause issues down the line
    use super::*;

    #[test]
    fn test_gap() {
        let mut blocks = Blocks::default();
        assert_eq!(0., blocks.insert_size(1_u8, 3.));
        assert_eq!(3., blocks.insert_size(2, 2.0));
    }
    #[test]
    fn test_reinsertion() {
        let mut blocks = Blocks::default();
        blocks.insert_size(1_u8, 3.);
        blocks.insert_size(2, 2.);
        blocks.insert_size(3, 8.);
        blocks.remove(2);
        assert_eq!(3., blocks.insert_size(4, 1.));
        assert_eq!(3. + 1., blocks.insert_size(5, 1.));
    }
    #[test]
    fn test_reinsertion_merging() {
        let mut blocks = Blocks::default();
        blocks.insert_size(1_u8, 3.);
        blocks.insert_size(2, 1.);
        blocks.insert_size(3, 1.);
        blocks.insert_size(4, 8.);
        blocks.remove(2);
        blocks.remove(3);
        assert_eq!(3., blocks.insert_size(5, 2.));
    }
    #[test]
    fn test_fat_block() {
        let mut blocks = Blocks::default();
        blocks.insert_size(1_u8, 1.);
        blocks.insert_size(2, 1.);
        blocks.insert_size(3, 8.);
        blocks.remove(2);
        assert_eq!(1. + 1. + 8., blocks.insert_size(4, 3.));
    }
    #[test]
    fn test_cleanup_single_block_end() {
        let mut blocks = Blocks::default();
        blocks.insert_size(1_u8, 1.);
        blocks.insert_size(2, 1.);
        blocks.remove(2);
        assert_eq!(1., blocks.insert_size(3, 1.0));
    }
    #[test]
    fn test_cleanup_multiple_block_end() {
        let mut blocks = Blocks::default();
        blocks.insert_size(1_u8, 1.);
        blocks.insert_size(2, 1.);
        blocks.insert_size(3, 1.);
        blocks.remove(2);
        blocks.remove(3);
        assert_eq!(1., blocks.insert_size(4, 1.));
    }
}
