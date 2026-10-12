//! Term dictionary: sorted terms in front-coded blocks of [`DICT_BLOCK`].
//!
//! ```text
//! u64 n_terms
//! array<u32> block_offs   n_blocks + 1 byte offsets into `blocks`
//! array<u32> first_offs   n_blocks + 1 byte offsets into `firsts`
//! array<u64> post_base    postings offset of each block's first term
//! array<u64> pos_base     positions offset of each block's first term
//! array<u8>  firsts       every block's first term, concatenated
//! array<u8>  blocks       entries: [prefix len, suffix len, suffix] (not
//!                         for the first entry), df, postings length,
//!                         positions length (varints)
//! ```
//!
//! The first terms are the block index: a lookup binary-searches them in
//! place (nothing is built at open), then scans one block. A term's
//! postings start where the previous term's end, so entries store lengths
//! and offsets are summed within the block.

use posting_bench::bytes::{ByteReader, ByteWriter, OwnedBytes, Slice};

use super::codec::{get_varint, put_varint};
use crate::error::{corrupt, Result};

/// Terms per front-coded block.
pub const DICT_BLOCK: usize = 32;

/// What the dictionary knows about a term.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TermInfo {
    /// Rank of the term in sorted order.
    pub ord: u32,
    /// Document frequency (length of the posting list).
    pub df: u32,
    /// Byte range of the posting list in the postings section.
    pub post_off: u64,
    pub post_len: u64,
    /// Byte range of the positions in the positions section (empty without
    /// positions).
    pub pos_off: u64,
    pub pos_len: u64,
}

/// Writes a dictionary from terms added in increasing byte order.
#[derive(Default)]
pub struct DictBuilder {
    n: usize,
    block_offs: Vec<u32>,
    first_offs: Vec<u32>,
    post_base: Vec<u64>,
    pos_base: Vec<u64>,
    firsts: Vec<u8>,
    blocks: Vec<u8>,
    prev: Vec<u8>,
    post_off: u64,
    pos_off: u64,
}

impl DictBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the next term (strictly greater than the previous one) whose
    /// postings and positions occupy the next `post_len` and `pos_len` bytes.
    pub fn add(&mut self, term: &[u8], df: u32, post_len: u64, pos_len: u64) {
        assert!(self.n == 0 || term > &self.prev[..], "terms must be added in increasing order");
        if self.n.is_multiple_of(DICT_BLOCK) {
            self.block_offs.push(self.blocks.len() as u32);
            self.first_offs.push(self.firsts.len() as u32);
            self.post_base.push(self.post_off);
            self.pos_base.push(self.pos_off);
            self.firsts.extend_from_slice(term);
        } else {
            let prefix = term.iter().zip(&self.prev).take_while(|(a, b)| a == b).count();
            put_varint(&mut self.blocks, prefix as u64);
            put_varint(&mut self.blocks, (term.len() - prefix) as u64);
            self.blocks.extend_from_slice(&term[prefix..]);
        }
        put_varint(&mut self.blocks, df as u64);
        put_varint(&mut self.blocks, post_len);
        put_varint(&mut self.blocks, pos_len);
        self.prev.clear();
        self.prev.extend_from_slice(term);
        self.post_off += post_len;
        self.pos_off += pos_len;
        self.n += 1;
    }

    pub fn finish(mut self) -> Vec<u8> {
        assert!(self.blocks.len() < u32::MAX as usize && self.firsts.len() < u32::MAX as usize, "dictionary too large");
        self.block_offs.push(self.blocks.len() as u32);
        self.first_offs.push(self.firsts.len() as u32);
        let mut w = ByteWriter::new();
        w.u64(self.n as u64);
        w.array(&self.block_offs);
        w.array(&self.first_offs);
        w.array(&self.post_base);
        w.array(&self.pos_base);
        w.array(&self.firsts);
        // Entries are read without bounds checks on varint boundaries; the
        // trailing padding of `array` keeps reads of a truncated entry
        // inside the buffer.
        w.array(&self.blocks);
        w.finish()
    }
}

/// Length of the common prefix of `a` and `b`.
#[inline]
fn lcp(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

/// A zero-copy view of a dictionary written by [`DictBuilder`].
#[derive(Clone, Debug)]
pub struct Dict {
    n_terms: u32,
    block_offs: Slice<u32>,
    first_offs: Slice<u32>,
    post_base: Slice<u64>,
    pos_base: Slice<u64>,
    firsts: Slice<u8>,
    blocks: Slice<u8>,
}

impl Dict {
    pub fn open(bytes: OwnedBytes) -> Result<Self> {
        let mut r = ByteReader::new(bytes);
        let n_terms = r.u64().map_err(corrupt)?;
        let d = Dict {
            n_terms: u32::try_from(n_terms).map_err(corrupt)?,
            block_offs: r.array().map_err(corrupt)?,
            first_offs: r.array().map_err(corrupt)?,
            post_base: r.array().map_err(corrupt)?,
            pos_base: r.array().map_err(corrupt)?,
            firsts: r.array().map_err(corrupt)?,
            blocks: r.array().map_err(corrupt)?,
        };
        let nb = (n_terms as usize).div_ceil(DICT_BLOCK);
        let ok = d.block_offs.len() == nb + 1
            && d.first_offs.len() == nb + 1
            && d.post_base.len() == nb
            && d.pos_base.len() == nb
            && d.block_offs.windows(2).all(|w| w[0] <= w[1])
            && d.first_offs.windows(2).all(|w| w[0] <= w[1])
            && d.block_offs[nb] as usize == d.blocks.len()
            && d.first_offs[nb] as usize == d.firsts.len();
        if !ok {
            return Err(corrupt("inconsistent term dictionary"));
        }
        Ok(d)
    }

    pub fn n_terms(&self) -> u32 {
        self.n_terms
    }

    fn n_blocks(&self) -> usize {
        self.post_base.len()
    }

    fn first(&self, b: usize) -> &[u8] {
        &self.firsts[self.first_offs[b] as usize..self.first_offs[b + 1] as usize]
    }

    /// The term's entry, if present.
    ///
    /// Scans the block without rebuilding its terms: with `m` the common
    /// prefix of the current term (which is below `t`) and `t`, an entry
    /// sharing fewer than `m` bytes with its predecessor is above `t`, one
    /// sharing more is still below it, and only one sharing exactly `m`
    /// needs its suffix compared.
    pub fn get(&self, term: &str) -> Option<TermInfo> {
        let t = term.as_bytes();
        // Last block whose first term is <= t.
        let b = self.block_partition(|first| first <= t).checked_sub(1)?;
        let data = &self.blocks[..self.block_offs[b + 1] as usize];
        let mut pos = self.block_offs[b] as usize;
        let (mut post_off, mut pos_off) = (self.post_base[b], self.pos_base[b]);
        let first = self.first(b);
        let mut m = lcp(first, t);
        let mut equal = m == first.len() && m == t.len();
        let end = ((b + 1) * DICT_BLOCK).min(self.n_terms as usize);
        for ord in b * DICT_BLOCK..end {
            if ord % DICT_BLOCK != 0 {
                let p = get_varint(data, &mut pos) as usize;
                let len = get_varint(data, &mut pos) as usize;
                let suffix = &data[pos..pos + len];
                pos += len;
                match p.cmp(&m) {
                    std::cmp::Ordering::Less => return None,
                    std::cmp::Ordering::Greater => {}
                    std::cmp::Ordering::Equal => {
                        let rest = &t[m..];
                        let l = lcp(suffix, rest);
                        if l < suffix.len() && (l == rest.len() || suffix[l] > rest[l]) {
                            return None;
                        }
                        m += l;
                        equal = l == suffix.len() && l == rest.len();
                    }
                }
            }
            let df = get_varint(data, &mut pos) as u32;
            let post_len = get_varint(data, &mut pos);
            let pos_len = get_varint(data, &mut pos);
            if equal {
                return Some(TermInfo { ord: ord as u32, df, post_off, post_len, pos_off, pos_len });
            }
            post_off += post_len;
            pos_off += pos_len;
        }
        None
    }

    /// Up to `limit` terms starting with `prefix`, in order.
    pub fn prefix(&self, prefix: &str, limit: usize) -> Vec<(String, TermInfo)> {
        let p = prefix.as_bytes();
        let mut out = Vec::new();
        if limit == 0 {
            return out;
        }
        // Terms >= prefix start in the last block whose first term is < prefix.
        let b0 = self.block_partition(|first| first < p).saturating_sub(1);
        for b in b0..self.n_blocks() {
            let mut more = true;
            self.scan_block(b, |term, info| {
                if term.starts_with(p) {
                    out.push((String::from_utf8_lossy(term).into_owned(), info));
                    more = out.len() < limit;
                    more
                } else {
                    // Before the matching run: keep going; after it: stop.
                    more = term < p;
                    more
                }
            });
            if !more {
                break;
            }
        }
        out
    }

    /// Every term with its entry, in order.
    pub fn for_each(&self, mut f: impl FnMut(&[u8], TermInfo)) {
        for b in 0..self.n_blocks() {
            self.scan_block(b, |t, info| {
                f(t, info);
                true
            });
        }
    }

    /// Number of leading blocks whose first term satisfies `pred` (which
    /// must hold for a prefix of the blocks).
    fn block_partition(&self, pred: impl Fn(&[u8]) -> bool) -> usize {
        let (mut lo, mut hi) = (0, self.n_blocks());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if pred(self.first(mid)) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Decodes block `b` in order, calling `f(term, info)` until it returns
    /// false.
    fn scan_block(&self, b: usize, mut f: impl FnMut(&[u8], TermInfo) -> bool) {
        let data = &self.blocks[..self.block_offs[b + 1] as usize];
        let mut pos = self.block_offs[b] as usize;
        let mut term: Vec<u8> = self.first(b).to_vec();
        let (mut post_off, mut pos_off) = (self.post_base[b], self.pos_base[b]);
        let mut ord = (b * DICT_BLOCK) as u32;
        let end = ((b + 1) * DICT_BLOCK).min(self.n_terms as usize) as u32;
        while ord < end {
            if !(ord as usize).is_multiple_of(DICT_BLOCK) {
                let prefix = get_varint(data, &mut pos) as usize;
                let suffix = get_varint(data, &mut pos) as usize;
                term.truncate(prefix);
                term.extend_from_slice(&data[pos..pos + suffix]);
                pos += suffix;
            }
            let df = get_varint(data, &mut pos) as u32;
            let post_len = get_varint(data, &mut pos);
            let pos_len = get_varint(data, &mut pos);
            let info = TermInfo { ord, df, post_off, post_len, pos_off, pos_len };
            if !f(&term, info) {
                return;
            }
            post_off += post_len;
            pos_off += pos_len;
            ord += 1;
        }
    }

    /// Bytes of the dictionary's arrays (index and blocks).
    pub fn byte_len(&self) -> usize {
        self.block_offs.byte_len()
            + self.first_offs.byte_len()
            + self.post_base.byte_len()
            + self.pos_base.byte_len()
            + self.firsts.byte_len()
            + self.blocks.byte_len()
    }

    /// Whether every array is a view into the section (no copies).
    pub fn is_shared(&self) -> bool {
        matches!(self.block_offs, Slice::Shared { .. })
            && matches!(self.first_offs, Slice::Shared { .. })
            && matches!(self.post_base, Slice::Shared { .. })
            && matches!(self.pos_base, Slice::Shared { .. })
            && matches!(self.firsts, Slice::Shared { .. })
            && matches!(self.blocks, Slice::Shared { .. })
    }
}
