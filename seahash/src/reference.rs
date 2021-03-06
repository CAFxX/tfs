//! A slow, but clear reference implementation of SeaHash.
//!
//! # Specification
//!
//! The input buffer is padded with null bytes until the length is divisible by 8.
//!
//! We start out with state
//!
//! ```notest
//! a = 0x16f11fe89b0d677c
//! b = 0xb480a793d8e6c86c
//! c = 0x6fe2e5aaf078ebc9
//! d = 0x14f994a4c5259381
//! ```
//!
//! From the stream, we read one 64-bit block (in little-endian) at a time.  This number, `n`,
//! determines the new the new state by:
//!
//! ```notest
//! a' = b
//! b' = c
//! c' = d
//! d  = g(a ⊕ n)
//! ```
//!
//! `g(x)` is defined as `g(x) = h(j(h(j(x)))))` with `h(x) = x ≫ 32` and `j(x) ≡ px (mod 2^64)`
//! with `p = 0x7ed0e9fa0d94a33`.
//!
//! Let the final state be `(x, y, z, w)`. Then the final result is given by `H = g(x ⊕ y ⊕ z ⊕ w ⊕
//! l)` where `l` is the number of bytes in the original buffer.

use diffuse;

/// Read an integer in little-endian.
fn read_int(int: &[u8]) -> u64 {
    debug_assert!(int.len() <= 8, "The buffer length of the integer must be less than or equal to \
                  the one of an u64.");

    // Start at 0.
    let mut x = 0;
    for &i in int.iter().rev() {
        // Shift up a byte.
        x <<= 8;
        // Set the lower byte.
        x |= i as u64;
    }

    x
}

/// A hash state.
struct State {
    /// The state vector.
    vec: [u64; 4],
    /// The component of the state vector which is currently being modified.
    cur: usize,
}

impl State {
    /// Write a 64-bit integer to the state.
    fn write_u64(&mut self, x: u64) {
        // Mix it into the substate by XORing it.
        self.vec[self.cur] ^= x;
        // Diffuse the component to remove deterministic behavior and commutativity.
        self.vec[self.cur] = diffuse(self.vec[self.cur]);

        // Increment the cursor.
        self.cur += 1;
        // Wrap around.
        self.cur %= 4;
    }

    /// Calculate the final hash.
    fn finish(self, total: usize) -> u64 {
        // Even though XORing is commutative, it doesn't matter, because the state vector's initial
        // components are mutually distinct, and thus swapping even and odd chunks will affect the
        // result, because it is sensitive to the initial condition. To add discreteness, we
        // diffuse.
        diffuse(self.vec[0]
            ^ self.vec[1]
            ^ self.vec[2]
            ^ self.vec[3]
            // We XOR in the number of written bytes to make it zero-sensitive when excessive bytes
            // are written (0u32.0u8 ≠ 0u16.0u8).
            ^ total as u64
        )
    }

    fn with_seed(seed: u64) -> State {
        State {
            // These values are randomly generated, and can be changed to anything (you could make
            // the hash function keyed by replacing these.)
            vec: [
                seed,
                0xb480a793d8e6c86c,
                0x6fe2e5aaf078ebc9,
                0x14f994a4c5259381,
            ],
            // We start at the first component.
            cur: 0,
        }
    }
}

/// A reference implementation of SeaHash.
///
/// This is bloody slow when compared to the optimized version. This is because SeaHash was
/// specifically designed to take all sorts of hardware and software hacks into account to achieve
/// maximal performance, but this makes code significantly less readable. As such, this version has
/// only one goal: to make the algorithm readable and understandable.
pub fn hash(buf: &[u8]) -> u64 {
    hash_seeded(buf, 0x16f11fe89b0d677c)
}

/// The seeded version of the reference implementation.
pub fn hash_seeded(buf: &[u8], seed: u64) -> u64 {
    // Initialize the state.
    let mut state = State::with_seed(seed);

    // Partition the rounded down buffer to chunks of 8 bytes, and iterate over them. The last
    // block might not be 8 bytes long.
    for int in buf.chunks(8) {
        // Read the chunk into an integer and write into the state.
        state.write_u64(read_int(int));
    }

    // Finish the hash state and return the final value.
    state.finish(buf.len())
}
