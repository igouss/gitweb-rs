//! git's binary delta encoder — a byte-exact port of `diff-delta.c`.
//!
//! Given a source (pre-image) and target (post-image) byte buffer, [`diff_delta`]
//! produces the same opcode stream git's `create_delta()` emits — the delta used
//! by packfiles and `git diff --binary`. It is the primitive a byte-exact GIT
//! binary patch body needs: that body picks whichever of {deflated literal,
//! deflated delta} is shorter, so the delta must match git's exact greedy-match
//! bytes, not merely be a valid applyable delta.
//!
//! Lineage: Josh MacDonald's xdelta → Davide Libenzi's libxdiff → Nicolas Pitre's
//! `diff-delta.c`. A Rabin-fingerprint-indexed greedy copy/insert matcher. This
//! is NOT VCDIFF/RFC-3284.
//!
//! ## The format (mirrors `patch-delta.c`)
//! Header: source size then target size, each a little-endian LEB128 varint
//! (7 bits per byte, high bit = "more follows"). Then instructions:
//! - COPY (`0x80 | mask`): copies a run from the source. The low 7 bits select up
//!   to 4 little-endian offset bytes (bits `0x01..0x08`) and up to 3 size bytes
//!   (bits `0x10..0x40`); absent bytes are zero. A copy is capped at `0x10000`
//!   bytes; a size encoded as 0 means `0x10000`.
//! - INSERT (`0x01..=0x7f`): the opcode value is the count of literal bytes that
//!   follow inline. `0x00` is reserved/invalid.
//!
//! ## The rolling hash invariant
//! The fingerprint update `val = ((val << 8) | byte) ^ T[val >> RABIN_SHIFT]` keeps
//! `val < 2^31` for all inputs: `T[k]` has bit 31 set iff `k` is odd, so the bit
//! shifted into position 31 by `<< 8` is always cancelled, and every `U[k]` has
//! bit 31 clear. Hence `val >> 23` is always in `0..=255` and never indexes past
//! the 256-entry tables. (Verified mechanically against the C source.)
//!
//! Pure, framework-free, no I/O, no `unsafe`. git's `create_delta` is called here
//! with `max_size == 0` (its `diff_delta` default), so the size-bounded early-out
//! paths are omitted: with no size cap they never change a single output byte.

/// Maximum entries kept in one hash bucket; guards against pathological inputs
/// with terrible hash distribution (`diff-delta.c` `HASH_LIMIT`).
const HASH_LIMIT: usize = 64;

/// Rabin fingerprint shift; selects the top 8 bits of `val` to index `T`.
const RABIN_SHIFT: u32 = 23;

/// Rabin window: the index fingerprints 16-byte blocks of the source.
const RABIN_WINDOW: usize = 16;

/// A single indexed source block: `ptr` is the source offset a copy would start
/// at, `val` is the block's Rabin fingerprint.
#[derive(Clone, Copy)]
struct IndexEntry {
    ptr: usize,
    val: u32,
}

/// The source index: buckets of [`IndexEntry`] keyed by `fingerprint & hash_mask`,
/// each bucket ordered lowest-offset-first (so ties prefer the lower offset).
struct DeltaIndex {
    hash_mask: u32,
    buckets: Vec<Vec<IndexEntry>>,
}

/// Builds a delta from `source` (pre-image) to `target` (post-image), byte-exact
/// with git's `diff_delta`. Returns `None` when either buffer is empty — git
/// cannot index an empty source nor delta an empty target, and falls back to a
/// literal in both cases (file creation / deletion).
#[must_use]
pub fn diff_delta(source: &[u8], target: &[u8]) -> Option<Vec<u8>> {
    if source.is_empty() || target.is_empty() {
        return None;
    }
    let index: DeltaIndex = create_index(source);
    Some(create_delta(&index, source, target))
}

/// Builds a size-bounded delta — git's `diff_delta(..., max_size)`, the form the
/// `GIT binary patch` body calls with `max_size` set to the deflated literal's
/// size. Identical to [`diff_delta`] but yields `None` when the finished delta
/// would exceed `max_size` bytes, exactly as git's `create_delta` abandons a delta
/// that outgrows its cap and falls back to a literal. A `max_size` of `0` means
/// unbounded (git's sentinel), so [`diff_delta`] is this with `max_size == 0`.
///
/// git enforces the cap incrementally, bailing out the instant the output passes
/// `max_size`; because the output only grows, that early bail and a single check
/// on the finished length pick the same outcome — `None` iff the complete delta
/// exceeds `max_size` — so building the whole delta then comparing its length is
/// byte-exact with git (the abandoned bytes are discarded either way).
#[must_use]
pub fn diff_delta_bounded(source: &[u8], target: &[u8], max_size: usize) -> Option<Vec<u8>> {
    let delta: Vec<u8> = diff_delta(source, target)?;
    if max_size != 0 && delta.len() > max_size {
        return None;
    }
    Some(delta)
}

/// Appends `value` as a little-endian LEB128 varint (`diff-delta.c` header form).
fn write_varint(out: &mut Vec<u8>, mut value: usize) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Builds the Rabin-fingerprint index of the (non-empty) source, mirroring
/// `create_delta_index`: 16-byte blocks fingerprinted back-to-front so the lowest
/// offset wins on ties, consecutive identical blocks coalesced to the lowest, and
/// over-dense buckets culled to `HASH_LIMIT` entries.
fn create_index(source: &[u8]) -> DeltaIndex {
    let bufsize: usize = source.len();

    // Indexing skips the first byte (optimises the Rabin init in create_delta).
    let mut entries: usize = (bufsize - 1) / RABIN_WINDOW;
    if bufsize >= 0xffff_ffff {
        // The delta format can't encode source offsets beyond 32 bits.
        entries = 0xffff_fffe / RABIN_WINDOW;
    }

    // Hash size: the smallest power of two >= entries/4, but never below 2^4.
    let bucket_target: usize = entries / 4;
    let mut shift: u32 = 4;
    while (1usize << shift) < bucket_target {
        shift += 1;
    }
    let hsize: usize = 1usize << shift;
    let hmask: u32 = (hsize - 1) as u32;

    // Linked-list index (pointers replaced by indices into `pool`).
    let mut pool: Vec<IndexEntry> = Vec::with_capacity(entries);
    let mut next: Vec<Option<usize>> = Vec::with_capacity(entries);
    let mut head: Vec<Option<usize>> = vec![None; hsize];
    let mut count: Vec<usize> = vec![0; hsize];

    let mut prev_val: u32 = !0;
    let mut off: isize = entries as isize * RABIN_WINDOW as isize - RABIN_WINDOW as isize;
    while off >= 0 {
        let data: usize = off as usize;
        let mut val: u32 = 0;
        for j in 1..=RABIN_WINDOW {
            val = ((val << 8) | source[data + j] as u32) ^ T[(val >> RABIN_SHIFT) as usize];
        }
        if val == prev_val {
            // Keep the lowest of a run of consecutive identical blocks.
            if let Some(last) = pool.last_mut() {
                last.ptr = data + RABIN_WINDOW;
            }
        } else {
            prev_val = val;
            let bucket: usize = (val & hmask) as usize;
            let idx: usize = pool.len();
            pool.push(IndexEntry {
                ptr: data + RABIN_WINDOW,
                val,
            });
            next.push(head[bucket]);
            head[bucket] = Some(idx);
            count[bucket] += 1;
        }
        off -= RABIN_WINDOW as isize;
    }

    cull_buckets(&mut head, &mut next, &count);

    let mut buckets: Vec<Vec<IndexEntry>> = vec![Vec::new(); hsize];
    for bucket in 0..hsize {
        let mut entry: Option<usize> = head[bucket];
        while let Some(idx) = entry {
            buckets[bucket].push(pool[idx]);
            entry = next[idx];
        }
    }

    DeltaIndex {
        hash_mask: hmask,
        buckets,
    }
}

/// Culls every bucket with more than `HASH_LIMIT` entries down to exactly
/// `HASH_LIMIT`, sampling uniformly across the source — the load-bearing
/// `create_delta_index` pruning that bounds match cost on pathological inputs.
fn cull_buckets(head: &mut [Option<usize>], next: &mut [Option<usize>], count: &[usize]) {
    for bucket in 0..head.len() {
        if count[bucket] <= HASH_LIMIT {
            continue;
        }
        let per: i64 = count[bucket] as i64 - HASH_LIMIT as i64;
        let mut acc: i64 = 0;
        let mut entry: Option<usize> = head[bucket];
        loop {
            acc += per;
            if acc > 0 {
                let keep: usize = entry.expect("cull keep: list shorter than count");
                loop {
                    entry = next[entry.expect("cull advance: list shorter than count")];
                    acc -= HASH_LIMIT as i64;
                    if acc <= 0 {
                        break;
                    }
                }
                next[keep] = next[entry.expect("cull splice: list shorter than count")];
            }
            entry = next[entry.expect("cull step: list shorter than count")];
            if entry.is_none() {
                break;
            }
        }
    }
}

/// Scans the source blocks hashing to `val` for the longest run that matches the
/// target at `d`, returning the best `(msize, moff)` — unchanged from the passed-in
/// floor if nothing longer is found. Buckets are lowest-offset-first, so ties keep
/// the lower offset, and the scan early-outs once a match reaches 4096 bytes.
fn find_longest_match(
    index: &DeltaIndex,
    source: &[u8],
    target: &[u8],
    d: usize,
    val: u32,
    mut msize: u64,
    mut moff: u64,
) -> (u64, u64) {
    let src_size: usize = source.len();
    let trg_size: usize = target.len();
    let bucket: usize = (val & index.hash_mask) as usize;
    for entry in &index.buckets[bucket] {
        if entry.val != val {
            continue;
        }
        let ref_off: usize = entry.ptr;
        let mut ref_size: usize = src_size - ref_off;
        let trg_rem: usize = trg_size - d;
        if ref_size > trg_rem {
            ref_size = trg_rem;
        }
        if ref_size as u64 <= msize {
            break;
        }
        let mut r: usize = ref_off;
        let mut s: usize = d;
        let mut rem: usize = ref_size;
        while rem > 0 && source[r] == target[s] {
            r += 1;
            s += 1;
            rem -= 1;
        }
        let matched: u64 = (r - ref_off) as u64;
        if msize < matched {
            msize = matched;
            moff = ref_off as u64;
            if msize >= 4096 {
                break;
            }
        }
    }
    (msize, moff)
}

/// Appends one COPY op: the `0x80` opcode whose low bits flag which little-endian
/// offset (`0x01..0x08`) and size (`0x10..0x20`) bytes are present, followed by
/// exactly those bytes. A zero size byte is omitted (the decoder reads it as
/// `0x10000`); callers cap `msize` at `0x10000` before calling.
fn emit_copy(out: &mut Vec<u8>, moff: u64, msize: u64) {
    let op_index: usize = out.len();
    out.push(0);
    let mut opcode: u32 = 0x80;
    if moff & 0x0000_00ff != 0 {
        out.push(moff as u8);
        opcode |= 0x01;
    }
    if moff & 0x0000_ff00 != 0 {
        out.push((moff >> 8) as u8);
        opcode |= 0x02;
    }
    if moff & 0x00ff_0000 != 0 {
        out.push((moff >> 16) as u8);
        opcode |= 0x04;
    }
    if moff & 0xff00_0000 != 0 {
        out.push((moff >> 24) as u8);
        opcode |= 0x08;
    }
    if msize & 0x0000_00ff != 0 {
        out.push(msize as u8);
        opcode |= 0x10;
    }
    if msize & 0x0000_ff00 != 0 {
        out.push((msize >> 8) as u8);
        opcode |= 0x20;
    }
    out[op_index] = opcode as u8;
}

/// Emits the delta from `index`/`source` to `target`, mirroring `create_delta`
/// with `max_size == 0` (no size-bounded early-out paths).
fn create_delta(index: &DeltaIndex, source: &[u8], target: &[u8]) -> Vec<u8> {
    let src_size: usize = source.len();
    let trg_size: usize = target.len();
    let mut out: Vec<u8> = Vec::new();

    write_varint(&mut out, src_size);
    write_varint(&mut out, trg_size);

    // Reserve the first insert-count slot, then prime the Rabin window with up to
    // RABIN_WINDOW literal bytes.
    out.push(0);
    let mut val: u32 = 0;
    let mut d: usize = 0;
    let prime: usize = core::cmp::min(RABIN_WINDOW, trg_size);
    for _ in 0..prime {
        out.push(target[d]);
        val = ((val << 8) | target[d] as u32) ^ T[(val >> RABIN_SHIFT) as usize];
        d += 1;
    }
    let mut inscnt: i64 = prime as i64;

    let mut moff: u64 = 0;
    let mut msize: u64 = 0;
    while d < trg_size {
        if msize < 4096 {
            // Roll the window: drop the byte that left it, fold in target[d].
            val ^= U[target[d - RABIN_WINDOW] as usize];
            val = ((val << 8) | target[d] as u32) ^ T[(val >> RABIN_SHIFT) as usize];
            (msize, moff) = find_longest_match(index, source, target, d, val, msize, moff);
        }

        if msize < 4 {
            if inscnt == 0 {
                out.push(0); // reserve a fresh insert-count slot
            }
            out.push(target[d]);
            d += 1;
            inscnt += 1;
            if inscnt == 0x7f {
                let slot: usize = out.len() - inscnt as usize - 1;
                out[slot] = inscnt as u8;
                inscnt = 0;
            }
            msize = 0;
        } else {
            if inscnt != 0 {
                // Back-extend the copy by stealing trailing insert bytes that also
                // match the source bytes just before the matched run.
                while moff > 0 && source[(moff - 1) as usize] == target[d - 1] {
                    msize += 1;
                    moff -= 1;
                    d -= 1;
                    out.pop(); // drop the literal byte
                    inscnt -= 1;
                    if inscnt != 0 {
                        continue;
                    }
                    out.pop(); // drop the now-empty count slot
                    inscnt = -1; // sentinel: slot already removed
                    break;
                }
                if inscnt >= 0 {
                    let slot: usize = out.len() - inscnt as usize - 1;
                    out[slot] = inscnt as u8;
                }
                inscnt = 0;
            }

            // A copy op is capped at 64KB; the remainder is re-looped.
            let left: u64 = msize.saturating_sub(0x10000);
            msize -= left;

            emit_copy(&mut out, moff, msize);

            d += msize as usize;
            moff += msize;
            msize = left;

            if moff > 0xffff_ffff {
                msize = 0;
            }

            if msize < 4096 {
                val = 0;
                for &byte in &target[(d - RABIN_WINDOW)..d] {
                    val = ((val << 8) | byte as u32) ^ T[(val >> RABIN_SHIFT) as usize];
                }
            }
        }
    }

    if inscnt != 0 {
        let slot: usize = out.len() - inscnt as usize - 1;
        out[slot] = inscnt as u8;
    }

    out
}

const T: [u32; 256] = [
    0x00000000, 0xab59b4d1, 0x56b369a2, 0xfdeadd73, 0x063f6795, 0xad66d344, 0x508c0e37, 0xfbd5bae6,
    0x0c7ecf2a, 0xa7277bfb, 0x5acda688, 0xf1941259, 0x0a41a8bf, 0xa1181c6e, 0x5cf2c11d, 0xf7ab75cc,
    0x18fd9e54, 0xb3a42a85, 0x4e4ef7f6, 0xe5174327, 0x1ec2f9c1, 0xb59b4d10, 0x48719063, 0xe32824b2,
    0x1483517e, 0xbfdae5af, 0x423038dc, 0xe9698c0d, 0x12bc36eb, 0xb9e5823a, 0x440f5f49, 0xef56eb98,
    0x31fb3ca8, 0x9aa28879, 0x6748550a, 0xcc11e1db, 0x37c45b3d, 0x9c9defec, 0x6177329f, 0xca2e864e,
    0x3d85f382, 0x96dc4753, 0x6b369a20, 0xc06f2ef1, 0x3bba9417, 0x90e320c6, 0x6d09fdb5, 0xc6504964,
    0x2906a2fc, 0x825f162d, 0x7fb5cb5e, 0xd4ec7f8f, 0x2f39c569, 0x846071b8, 0x798aaccb, 0xd2d3181a,
    0x25786dd6, 0x8e21d907, 0x73cb0474, 0xd892b0a5, 0x23470a43, 0x881ebe92, 0x75f463e1, 0xdeadd730,
    0x63f67950, 0xc8afcd81, 0x354510f2, 0x9e1ca423, 0x65c91ec5, 0xce90aa14, 0x337a7767, 0x9823c3b6,
    0x6f88b67a, 0xc4d102ab, 0x393bdfd8, 0x92626b09, 0x69b7d1ef, 0xc2ee653e, 0x3f04b84d, 0x945d0c9c,
    0x7b0be704, 0xd05253d5, 0x2db88ea6, 0x86e13a77, 0x7d348091, 0xd66d3440, 0x2b87e933, 0x80de5de2,
    0x7775282e, 0xdc2c9cff, 0x21c6418c, 0x8a9ff55d, 0x714a4fbb, 0xda13fb6a, 0x27f92619, 0x8ca092c8,
    0x520d45f8, 0xf954f129, 0x04be2c5a, 0xafe7988b, 0x5432226d, 0xff6b96bc, 0x02814bcf, 0xa9d8ff1e,
    0x5e738ad2, 0xf52a3e03, 0x08c0e370, 0xa39957a1, 0x584ced47, 0xf3155996, 0x0eff84e5, 0xa5a63034,
    0x4af0dbac, 0xe1a96f7d, 0x1c43b20e, 0xb71a06df, 0x4ccfbc39, 0xe79608e8, 0x1a7cd59b, 0xb125614a,
    0x468e1486, 0xedd7a057, 0x103d7d24, 0xbb64c9f5, 0x40b17313, 0xebe8c7c2, 0x16021ab1, 0xbd5bae60,
    0x6cb54671, 0xc7ecf2a0, 0x3a062fd3, 0x915f9b02, 0x6a8a21e4, 0xc1d39535, 0x3c394846, 0x9760fc97,
    0x60cb895b, 0xcb923d8a, 0x3678e0f9, 0x9d215428, 0x66f4eece, 0xcdad5a1f, 0x3047876c, 0x9b1e33bd,
    0x7448d825, 0xdf116cf4, 0x22fbb187, 0x89a20556, 0x7277bfb0, 0xd92e0b61, 0x24c4d612, 0x8f9d62c3,
    0x7836170f, 0xd36fa3de, 0x2e857ead, 0x85dcca7c, 0x7e09709a, 0xd550c44b, 0x28ba1938, 0x83e3ade9,
    0x5d4e7ad9, 0xf617ce08, 0x0bfd137b, 0xa0a4a7aa, 0x5b711d4c, 0xf028a99d, 0x0dc274ee, 0xa69bc03f,
    0x5130b5f3, 0xfa690122, 0x0783dc51, 0xacda6880, 0x570fd266, 0xfc5666b7, 0x01bcbbc4, 0xaae50f15,
    0x45b3e48d, 0xeeea505c, 0x13008d2f, 0xb85939fe, 0x438c8318, 0xe8d537c9, 0x153feaba, 0xbe665e6b,
    0x49cd2ba7, 0xe2949f76, 0x1f7e4205, 0xb427f6d4, 0x4ff24c32, 0xe4abf8e3, 0x19412590, 0xb2189141,
    0x0f433f21, 0xa41a8bf0, 0x59f05683, 0xf2a9e252, 0x097c58b4, 0xa225ec65, 0x5fcf3116, 0xf49685c7,
    0x033df00b, 0xa86444da, 0x558e99a9, 0xfed72d78, 0x0502979e, 0xae5b234f, 0x53b1fe3c, 0xf8e84aed,
    0x17bea175, 0xbce715a4, 0x410dc8d7, 0xea547c06, 0x1181c6e0, 0xbad87231, 0x4732af42, 0xec6b1b93,
    0x1bc06e5f, 0xb099da8e, 0x4d7307fd, 0xe62ab32c, 0x1dff09ca, 0xb6a6bd1b, 0x4b4c6068, 0xe015d4b9,
    0x3eb80389, 0x95e1b758, 0x680b6a2b, 0xc352defa, 0x3887641c, 0x93ded0cd, 0x6e340dbe, 0xc56db96f,
    0x32c6cca3, 0x999f7872, 0x6475a501, 0xcf2c11d0, 0x34f9ab36, 0x9fa01fe7, 0x624ac294, 0xc9137645,
    0x26459ddd, 0x8d1c290c, 0x70f6f47f, 0xdbaf40ae, 0x207afa48, 0x8b234e99, 0x76c993ea, 0xdd90273b,
    0x2a3b52f7, 0x8162e626, 0x7c883b55, 0xd7d18f84, 0x2c043562, 0x875d81b3, 0x7ab75cc0, 0xd1eee811,
];

const U: [u32; 256] = [
    0x00000000, 0x7eb5200d, 0x5633f4cb, 0x2886d4c6, 0x073e5d47, 0x798b7d4a, 0x510da98c, 0x2fb88981,
    0x0e7cba8e, 0x70c99a83, 0x584f4e45, 0x26fa6e48, 0x0942e7c9, 0x77f7c7c4, 0x5f711302, 0x21c4330f,
    0x1cf9751c, 0x624c5511, 0x4aca81d7, 0x347fa1da, 0x1bc7285b, 0x65720856, 0x4df4dc90, 0x3341fc9d,
    0x1285cf92, 0x6c30ef9f, 0x44b63b59, 0x3a031b54, 0x15bb92d5, 0x6b0eb2d8, 0x4388661e, 0x3d3d4613,
    0x39f2ea38, 0x4747ca35, 0x6fc11ef3, 0x11743efe, 0x3eccb77f, 0x40799772, 0x68ff43b4, 0x164a63b9,
    0x378e50b6, 0x493b70bb, 0x61bda47d, 0x1f088470, 0x30b00df1, 0x4e052dfc, 0x6683f93a, 0x1836d937,
    0x250b9f24, 0x5bbebf29, 0x73386bef, 0x0d8d4be2, 0x2235c263, 0x5c80e26e, 0x740636a8, 0x0ab316a5,
    0x2b7725aa, 0x55c205a7, 0x7d44d161, 0x03f1f16c, 0x2c4978ed, 0x52fc58e0, 0x7a7a8c26, 0x04cfac2b,
    0x73e5d470, 0x0d50f47d, 0x25d620bb, 0x5b6300b6, 0x74db8937, 0x0a6ea93a, 0x22e87dfc, 0x5c5d5df1,
    0x7d996efe, 0x032c4ef3, 0x2baa9a35, 0x551fba38, 0x7aa733b9, 0x041213b4, 0x2c94c772, 0x5221e77f,
    0x6f1ca16c, 0x11a98161, 0x392f55a7, 0x479a75aa, 0x6822fc2b, 0x1697dc26, 0x3e1108e0, 0x40a428ed,
    0x61601be2, 0x1fd53bef, 0x3753ef29, 0x49e6cf24, 0x665e46a5, 0x18eb66a8, 0x306db26e, 0x4ed89263,
    0x4a173e48, 0x34a21e45, 0x1c24ca83, 0x6291ea8e, 0x4d29630f, 0x339c4302, 0x1b1a97c4, 0x65afb7c9,
    0x446b84c6, 0x3adea4cb, 0x1258700d, 0x6ced5000, 0x4355d981, 0x3de0f98c, 0x15662d4a, 0x6bd30d47,
    0x56ee4b54, 0x285b6b59, 0x00ddbf9f, 0x7e689f92, 0x51d01613, 0x2f65361e, 0x07e3e2d8, 0x7956c2d5,
    0x5892f1da, 0x2627d1d7, 0x0ea10511, 0x7014251c, 0x5facac9d, 0x21198c90, 0x099f5856, 0x772a785b,
    0x4c921c31, 0x32273c3c, 0x1aa1e8fa, 0x6414c8f7, 0x4bac4176, 0x3519617b, 0x1d9fb5bd, 0x632a95b0,
    0x42eea6bf, 0x3c5b86b2, 0x14dd5274, 0x6a687279, 0x45d0fbf8, 0x3b65dbf5, 0x13e30f33, 0x6d562f3e,
    0x506b692d, 0x2ede4920, 0x06589de6, 0x78edbdeb, 0x5755346a, 0x29e01467, 0x0166c0a1, 0x7fd3e0ac,
    0x5e17d3a3, 0x20a2f3ae, 0x08242768, 0x76910765, 0x59298ee4, 0x279caee9, 0x0f1a7a2f, 0x71af5a22,
    0x7560f609, 0x0bd5d604, 0x235302c2, 0x5de622cf, 0x725eab4e, 0x0ceb8b43, 0x246d5f85, 0x5ad87f88,
    0x7b1c4c87, 0x05a96c8a, 0x2d2fb84c, 0x539a9841, 0x7c2211c0, 0x029731cd, 0x2a11e50b, 0x54a4c506,
    0x69998315, 0x172ca318, 0x3faa77de, 0x411f57d3, 0x6ea7de52, 0x1012fe5f, 0x38942a99, 0x46210a94,
    0x67e5399b, 0x19501996, 0x31d6cd50, 0x4f63ed5d, 0x60db64dc, 0x1e6e44d1, 0x36e89017, 0x485db01a,
    0x3f77c841, 0x41c2e84c, 0x69443c8a, 0x17f11c87, 0x38499506, 0x46fcb50b, 0x6e7a61cd, 0x10cf41c0,
    0x310b72cf, 0x4fbe52c2, 0x67388604, 0x198da609, 0x36352f88, 0x48800f85, 0x6006db43, 0x1eb3fb4e,
    0x238ebd5d, 0x5d3b9d50, 0x75bd4996, 0x0b08699b, 0x24b0e01a, 0x5a05c017, 0x728314d1, 0x0c3634dc,
    0x2df207d3, 0x534727de, 0x7bc1f318, 0x0574d315, 0x2acc5a94, 0x54797a99, 0x7cffae5f, 0x024a8e52,
    0x06852279, 0x78300274, 0x50b6d6b2, 0x2e03f6bf, 0x01bb7f3e, 0x7f0e5f33, 0x57888bf5, 0x293dabf8,
    0x08f998f7, 0x764cb8fa, 0x5eca6c3c, 0x207f4c31, 0x0fc7c5b0, 0x7172e5bd, 0x59f4317b, 0x27411176,
    0x1a7c5765, 0x64c97768, 0x4c4fa3ae, 0x32fa83a3, 0x1d420a22, 0x63f72a2f, 0x4b71fee9, 0x35c4dee4,
    0x1400edeb, 0x6ab5cde6, 0x42331920, 0x3c86392d, 0x133eb0ac, 0x6d8b90a1, 0x450d4467, 0x3bb8646a,
];
