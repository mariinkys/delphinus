use memmap2::{Advice, Mmap, UncheckedAdvice};
use std::{cmp::Ordering, fs::File, io, path::Path};

/// A read-only memory-mapped file
pub struct Mapped {
    map: Mmap,
}

impl Mapped {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        if len == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "file is empty"));
        }
        if len >= u32::MAX as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file larger than 4 GiB",
            ));
        }
        // SAFETY: the map is read-only and the file is never modified by this process
        let map = unsafe { Mmap::map(&file)? };
        let _ = map.advise(Advice::Sequential);
        Ok(Self { map })
    }

    #[inline]
    pub fn bytes(&self) -> &[u8] {
        &self.map
    }

    /// Called once the start-up scan is done: drop the scanned pages from our
    /// resident set (they stay in the page cache) and stop read-ahead
    pub fn settle(&self) {
        self.release();
        let _ = self.map.advise(Advice::Random);
    }

    pub fn release(&self) {
        // SAFETY: MADV_DONTNEED on a read-only, file-backed shared mapping only
        // drops page-table entries; later reads fault the same data back in
        unsafe {
            let _ = self.map.unchecked_advise(UncheckedAdvice::DontNeed);
        }
    }
}

/// A sorted list of byte offsets into a [`Mapped`] file. Each offset is the
/// start of a key; `key_at` knows where that key ends
pub struct KeyIndex {
    offsets: Box<[u32]>,
}

impl KeyIndex {
    pub fn build(data: &[u8], mut offsets: Vec<u32>, key_at: fn(&[u8], u32) -> &[u8]) -> Self {
        offsets.sort_unstable_by(|&a, &b| match key_at(data, a).cmp(key_at(data, b)) {
            Ordering::Equal => a.cmp(&b),
            o => o,
        });
        Self {
            offsets: offsets.into_boxed_slice(),
        }
    }

    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// All offsets whose key equals `key`, in file order
    pub fn find<'a>(
        &'a self,
        data: &[u8],
        key: &[u8],
        key_at: fn(&[u8], u32) -> &[u8],
    ) -> &'a [u32] {
        let lo = self.offsets.partition_point(|&o| key_at(data, o) < key);
        let n = self.offsets[lo..].partition_point(|&o| key_at(data, o) == key);
        &self.offsets[lo..lo + n]
    }
}
