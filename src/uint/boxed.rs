//! Heap-allocated big unsigned integers.

mod add;
mod add_mod;
mod bit_and;
mod bits;
mod cmp;
pub(crate) mod encoding;
mod modular;
mod mul;
mod shl;
mod shr;
mod sub;
mod sub_mod;

use crate::{Limb, Uint, Word, Zero, U128, U64};
use alloc::{boxed::Box, vec, vec::Vec};
use core::fmt;
use subtle::{Choice, ConditionallySelectable};

#[cfg(feature = "zeroize")]
use zeroize::Zeroize;

/// Fixed-precision heap-allocated big unsigned integer.
///
/// Alternative to the stack-allocated [`Uint`][`crate::Uint`] but with a
/// fixed precision chosen at runtime instead of compile time.
///
/// Unlike many other heap-allocated big integer libraries, this type is not
/// arbitrary precision and will wrap at its fixed-precision rather than
/// automatically growing.
#[derive(Clone, Default)]
pub struct BoxedUint {
    /// Boxed slice containing limbs.
    ///
    /// Stored from least significant to most significant.
    limbs: Box<[Limb]>,
}

impl BoxedUint {
    /// Get the value `0` represented as succinctly as possible.
    pub fn zero() -> Self {
        Self::default()
    }

    /// Get the value `0` with the given number of bits of precision.
    ///
    /// Panics if the precision is not a multiple of [`Limb::BITS`].
    pub fn zero_with_precision(bits_precision: usize) -> Self {
        assert_eq!(
            bits_precision % Limb::BITS,
            0,
            "precision is not a multiple of limb size"
        );

        vec![Limb::ZERO; bits_precision / Limb::BITS].into()
    }

    /// Get the value `1`, represented as succinctly as possible.
    pub fn one() -> Self {
        Self {
            limbs: vec![Limb::ONE; 1].into(),
        }
    }

    /// Get the value `1` with the given number of bits of precision.
    ///
    /// Panics if the precision is not at least [`Limb::BITS`] or if it is not
    /// a multiple thereof.
    pub fn one_with_precision(bits_precision: usize) -> Self {
        assert!(bits_precision >= Limb::BITS, "precision too small");
        let mut ret = Self::zero_with_precision(bits_precision);
        ret.limbs[0] = Limb::ONE;
        ret
    }

    /// Is this [`BoxedUint`] equal to zero?
    pub fn is_zero(&self) -> Choice {
        self.limbs
            .iter()
            .fold(Choice::from(1), |acc, limb| acc & limb.is_zero())
    }

    /// Get the maximum value for a given number of bits of precision.
    ///
    /// Panics if the precision is not a multiple of [`Limb::BITS`].
    pub fn max(bits_precision: usize) -> Self {
        assert_eq!(
            bits_precision % Limb::BITS,
            0,
            "precision is not a multiple of limb size"
        );

        vec![Limb::MAX; bits_precision / Limb::BITS].into()
    }

    /// Create a [`BoxedUint`] from an array of [`Word`]s (i.e. word-sized unsigned
    /// integers).
    #[inline]
    pub fn from_words(words: impl IntoIterator<Item = Word>) -> Self {
        Self {
            limbs: words.into_iter().map(Into::into).collect(),
        }
    }

    /// Create a boxed slice of [`Word`]s (i.e. word-sized unsigned integers) from
    /// a [`BoxedUint`].
    #[inline]
    pub fn to_words(&self) -> Box<[Word]> {
        self.limbs.iter().copied().map(Into::into).collect()
    }

    /// Borrow the inner limbs as a slice of [`Word`]s.
    pub fn as_words(&self) -> &[Word] {
        // SAFETY: `Limb` is a `repr(transparent)` newtype for `Word`
        #[allow(trivial_casts, unsafe_code)]
        unsafe {
            &*((&*self.limbs as *const _) as *const [Word])
        }
    }

    /// Borrow the inner limbs as a mutable slice of [`Word`]s.
    pub fn as_words_mut(&mut self) -> &mut [Word] {
        // SAFETY: `Limb` is a `repr(transparent)` newtype for `Word`
        #[allow(trivial_casts, unsafe_code)]
        unsafe {
            &mut *((&mut *self.limbs as *mut _) as *mut [Word])
        }
    }

    /// Borrow the limbs of this [`BoxedUint`].
    pub fn as_limbs(&self) -> &[Limb] {
        self.limbs.as_ref()
    }

    /// Borrow the limbs of this [`BoxedUint`] mutably.
    pub fn as_limbs_mut(&mut self) -> &mut [Limb] {
        self.limbs.as_mut()
    }

    /// Convert this [`BoxedUint`] into its inner limbs.
    pub fn to_limbs(&self) -> Box<[Limb]> {
        self.limbs.clone()
    }

    /// Convert this [`BoxedUint`] into its inner limbs.
    pub fn into_limbs(self) -> Box<[Limb]> {
        self.limbs
    }

    /// Get the number of limbs in this [`BoxedUint`].
    pub fn nlimbs(&self) -> usize {
        self.limbs.len()
    }

    /// Conditionally select `a` or `b` in constant time depending on [`Choice`].
    ///
    /// NOTE: can't impl `subtle`'s [`ConditionallySelectable`] trait due to its `Copy` bound, so
    /// this is an inherent function instead.
    ///
    /// Panics if `a` and `b` don't have the same precision.
    pub fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        debug_assert_eq!(a.nlimbs(), b.nlimbs());
        let mut limbs = vec![Limb::ZERO; a.nlimbs()].into_boxed_slice();

        for i in 0..a.nlimbs() {
            limbs[i] = Limb::conditional_select(&a.limbs[i], &b.limbs[i], choice);
        }

        Self { limbs }
    }

    /// Perform a carry chain-like operation over the limbs of the inputs,
    /// constructing a result from the returned limbs and carry which is
    /// widened to the same width as the widest input.
    ///
    /// If one of the two values has fewer limbs than the other, pads with
    /// [`Limb::ZERO`] as the value for that limb.
    fn chain<F>(lhs: &Self, rhs: &Self, mut carry: Limb, f: F) -> (Self, Limb)
    where
        F: Fn(Limb, Limb, Limb) -> (Limb, Limb),
    {
        let nlimbs = cmp::max(lhs.nlimbs(), rhs.nlimbs());
        let mut limbs = Vec::with_capacity(nlimbs);

        for i in 0..nlimbs {
            let &a = lhs.limbs.get(i).unwrap_or(&Limb::ZERO);
            let &b = rhs.limbs.get(i).unwrap_or(&Limb::ZERO);
            let (limb, c) = f(a, b, carry);
            limbs.push(limb);
            carry = c;
        }

        (limbs.into(), carry)
    }
}

impl AsRef<[Word]> for BoxedUint {
    fn as_ref(&self) -> &[Word] {
        self.as_words()
    }
}

impl AsMut<[Word]> for BoxedUint {
    fn as_mut(&mut self) -> &mut [Word] {
        self.as_words_mut()
    }
}

impl AsRef<[Limb]> for BoxedUint {
    fn as_ref(&self) -> &[Limb] {
        self.as_limbs()
    }
}

impl AsMut<[Limb]> for BoxedUint {
    fn as_mut(&mut self) -> &mut [Limb] {
        self.as_limbs_mut()
    }
}

impl fmt::Debug for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BoxedUint(0x{self:X})")
    }
}

impl fmt::Display for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(self, f)
    }
}

impl From<u8> for BoxedUint {
    fn from(n: u8) -> Self {
        vec![Limb::from(n); 1].into()
    }
}

impl From<u16> for BoxedUint {
    fn from(n: u16) -> Self {
        vec![Limb::from(n); 1].into()
    }
}

impl From<u32> for BoxedUint {
    fn from(n: u32) -> Self {
        vec![Limb::from(n); 1].into()
    }
}

impl From<u64> for BoxedUint {
    fn from(n: u64) -> Self {
        U64::from(n).into()
    }
}

impl From<u128> for BoxedUint {
    fn from(n: u128) -> Self {
        U128::from(n).into()
    }
}

impl From<&[Limb]> for BoxedUint {
    fn from(limbs: &[Limb]) -> BoxedUint {
        Self {
            limbs: limbs.into(),
        }
    }
}

impl From<Box<[Limb]>> for BoxedUint {
    fn from(limbs: Box<[Limb]>) -> BoxedUint {
        Self { limbs }
    }
}

impl From<Vec<Limb>> for BoxedUint {
    fn from(limbs: Vec<Limb>) -> BoxedUint {
        limbs.into_boxed_slice().into()
    }
}

impl<const LIMBS: usize> From<Uint<LIMBS>> for BoxedUint {
    fn from(uint: Uint<LIMBS>) -> BoxedUint {
        Vec::from(uint.to_limbs()).into()
    }
}

impl fmt::LowerHex for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.limbs.is_empty() {
            return fmt::LowerHex::fmt(&Limb::ZERO, f);
        }

        for limb in self.limbs.iter().rev() {
            fmt::LowerHex::fmt(limb, f)?;
        }
        Ok(())
    }
}

impl fmt::UpperHex for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.limbs.is_empty() {
            return fmt::LowerHex::fmt(&Limb::ZERO, f);
        }

        for limb in self.limbs.iter().rev() {
            fmt::UpperHex::fmt(limb, f)?;
        }
        Ok(())
    }
}

#[cfg(feature = "zeroize")]
impl Zeroize for BoxedUint {
    fn zeroize(&mut self) {
        self.limbs.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::BoxedUint;
    use subtle::Choice;

    #[test]
    fn conditional_select() {
        let a = BoxedUint::zero_with_precision(128);
        let b = BoxedUint::max(128);

        assert_eq!(a, BoxedUint::conditional_select(&a, &b, Choice::from(0)));
        assert_eq!(b, BoxedUint::conditional_select(&a, &b, Choice::from(1)));
    }
}