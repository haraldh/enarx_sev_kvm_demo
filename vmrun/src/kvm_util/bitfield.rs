use sparse_bitfield::Bitfield as SparseBitfield;
use std::ops::Range;

pub struct Rangefield {
    inner: Vec<Range<usize>>,
}
impl Default for Rangefield {
    fn default() -> Self {
        Rangefield { inner: vec![] }
    }
}
impl core::ops::Deref for Rangefield {
    type Target = Vec<Range<usize>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl core::ops::DerefMut for Rangefield {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl Rangefield {
    pub fn is_set_num(&self, start: usize, num: usize) -> bool {
        for r in self.inner.iter() {
            if !r.contains(&start) {
                continue;
            }
            if !r.contains(&(start + num - 1)) {
                continue;
            }
            return true;
        }
        return false;
    }
    pub fn next_set_num(&self, prev: usize, num: usize) -> Option<usize> {
        let start = prev + 1;
        for r in self.inner.iter() {
            if !r.start < start {
                continue;
            }
            if r.end < r.start + num {
                continue;
            }
            return Some(r.start);
        }
        return None;
    }
}

pub struct Bitfield {
    inner: SparseBitfield,
}

impl Bitfield {
    pub fn next_clear(&mut self, prev: usize) -> usize {
        use std::hint::unreachable_unchecked;

        for i in prev.wrapping_add(1)..std::usize::MAX {
            if !self.inner.get(i) {
                return i;
            }
        }
        // self.inner.get(self.inner.len()) is always unset
        unsafe { unreachable_unchecked() }
    }

    pub fn next_set(&mut self, prev: usize) -> Option<usize> {
        for i in prev + 1..std::usize::MAX {
            if self.inner.get(i) {
                return Some(i);
            }
        }

        None
    }

    pub fn is_clear_num(&mut self, start: usize, num: usize) -> bool {
        for i in start..start + num {
            if self.inner.get(i) {
                return false;
            }
        }

        true
    }

    pub fn next_clear_num(&mut self, prev: usize, num: usize) -> usize {
        use std::hint::unreachable_unchecked;

        let mut start = prev;

        loop {
            start = self.next_clear(start);
            if self.is_clear_num(start, num) {
                return start;
            }
            start = match self.next_set(start) {
                Some(start) => start,
                None => unsafe { unreachable_unchecked() },
            };
        }
    }

    pub fn set_range(&mut self, start: usize, num: usize) {
        for i in start..(start + num) {
            self.inner.set(i, true);
        }
    }

    pub fn find_unset_range(&mut self, start: usize, num: usize) -> Option<usize> {
        let mut start = start.wrapping_sub(1);

        'start_over: loop {
            start = self.next_clear(start);

            let end = start + num;
            let mut i = start;
            loop {
                if self.inner.get(i) {
                    start = i;
                    continue 'start_over;
                }
                i += 1;
                if i == end {
                    return Some(start);
                }
            }
        }
    }
}

impl core::ops::Deref for Bitfield {
    type Target = SparseBitfield;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl core::ops::DerefMut for Bitfield {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Default for Bitfield {
    fn default() -> Self {
        Bitfield {
            inner: SparseBitfield::default(),
        }
    }
}
