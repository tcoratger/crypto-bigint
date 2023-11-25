//! Const-friendly decoding operations for [`BoxedUint`].

use super::BoxedUint;
use crate::Limb;

/// Decoding errors for [`BoxedUint`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// Input is not a valid size.
    InputSize,

    /// Precision is not a multiple of [`Limb::BYTES`].
    Precision,
}

impl BoxedUint {
    /// Create a new [`BoxedUint`] from the provided big endian bytes.
    ///
    /// The `bits_precision` argument represents the precision of the resulting integer, which is
    /// fixed as this type is not arbitrary-precision. It MUST be a multiple of the limb size, i.e.
    /// [`Limb::BITS`], or otherwise this function will return [`DecodeError::Precision`].
    ///
    /// If the length of `bytes` (when interpreted as bits) is larger than `bits_precision`, this
    /// function will return [`DecodeError::InputSize`].
    pub fn from_be_slice(bytes: &[u8], bits_precision: usize) -> Result<Self, DecodeError> {
        if bits_precision % Limb::BITS != 0 {
            return Err(DecodeError::Precision);
        }

        if bytes.len() % Limb::BYTES != 0 || bytes.len() * 8 > bits_precision {
            return Err(DecodeError::InputSize);
        }

        let mut ret = Self::zero_with_precision(bits_precision);

        for (chunk, limb) in bytes.chunks(Limb::BYTES).rev().zip(ret.limbs.iter_mut()) {
            *limb = Limb::from_be_slice(chunk);
        }

        Ok(ret)
    }

    /// Create a new [`BoxedUint`] from the provided little endian bytes.
    ///
    /// The `bits_precision` argument represents the precision of the resulting integer, which is
    /// fixed as this type is not arbitrary-precision. It MUST be a multiple of the limb size, i.e.
    /// [`Limb::BITS`], or otherwise this function will return [`DecodeError::Precision`].
    ///
    /// If the length of `bytes` (when interpreted as bits) is larger than `bits_precision`, this
    /// function will return [`DecodeError::InputSize`].
    pub fn from_le_slice(bytes: &[u8], bits_precision: usize) -> Result<Self, DecodeError> {
        if bits_precision % Limb::BITS != 0 {
            return Err(DecodeError::Precision);
        }

        if bytes.len() % Limb::BYTES != 0 || bytes.len() * 8 > bits_precision {
            return Err(DecodeError::InputSize);
        }

        let mut ret = Self::zero_with_precision(bits_precision);

        for (chunk, limb) in bytes.chunks(Limb::BYTES).zip(ret.limbs.iter_mut()) {
            *limb = Limb::from_le_slice(chunk);
        }

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::{BoxedUint, DecodeError};
    use crate::Limb;
    use hex_literal::hex;

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn from_be_slice_eq() {
        let bytes = hex!("0011223344556677");
        let n = BoxedUint::from_be_slice(&bytes, 64).unwrap();
        assert_eq!(n.as_limbs(), &[Limb(0x44556677), Limb(0x00112233)]);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn from_be_slice_eq() {
        let bytes = hex!("00112233445566778899aabbccddeeff");
        let n = BoxedUint::from_be_slice(&bytes, 128).unwrap();
        assert_eq!(
            n.as_limbs(),
            &[Limb(0x8899aabbccddeeff), Limb(0x0011223344556677)]
        );
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn from_be_slice_short() {
        let bytes = hex!("0011223344556677");
        let n = BoxedUint::from_be_slice(&bytes, 128).unwrap();
        assert_eq!(
            n.as_limbs(),
            &[Limb(0x44556677), Limb(0x00112233), Limb::ZERO, Limb::ZERO]
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn from_be_slice_short() {
        let bytes = hex!("00112233445566778899aabbccddeeff");
        let n = BoxedUint::from_be_slice(&bytes, 256).unwrap();
        assert_eq!(
            n.as_limbs(),
            &[
                Limb(0x8899aabbccddeeff),
                Limb(0x0011223344556677),
                Limb::ZERO,
                Limb::ZERO
            ]
        );
    }

    #[test]
    fn from_be_slice_too_long() {
        let bytes = hex!("00112233445566778899aabbccddeeff");
        assert_eq!(
            BoxedUint::from_be_slice(&bytes, 64),
            Err(DecodeError::InputSize)
        );
    }

    #[test]
    fn from_be_slice_not_word_sized() {
        let bytes = hex!("00112233445566778899aabbccddee");
        assert_eq!(
            BoxedUint::from_be_slice(&bytes, 128),
            Err(DecodeError::InputSize)
        );
    }

    #[test]
    fn from_be_slice_bad_precision() {
        let bytes = hex!("00112233445566778899aabbccddeeff");
        assert_eq!(
            BoxedUint::from_be_slice(&bytes, 127),
            Err(DecodeError::Precision)
        );
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn from_le_slice_eq() {
        let bytes = hex!("7766554433221100");
        let n = BoxedUint::from_le_slice(&bytes, 64).unwrap();
        assert_eq!(n.as_limbs(), &[Limb(0x44556677), Limb(0x00112233)]);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn from_le_slice_eq() {
        let bytes = hex!("ffeeddccbbaa99887766554433221100");
        let n = BoxedUint::from_le_slice(&bytes, 128).unwrap();
        assert_eq!(
            n.as_limbs(),
            &[Limb(0x8899aabbccddeeff), Limb(0x0011223344556677)]
        );
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn from_le_slice_short() {
        let bytes = hex!("7766554433221100");
        let n = BoxedUint::from_le_slice(&bytes, 128).unwrap();
        assert_eq!(
            n.as_limbs(),
            &[Limb(0x44556677), Limb(0x00112233), Limb::ZERO, Limb::ZERO]
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn from_le_slice_short() {
        let bytes = hex!("ffeeddccbbaa99887766554433221100");
        let n = BoxedUint::from_le_slice(&bytes, 256).unwrap();
        assert_eq!(
            n.as_limbs(),
            &[
                Limb(0x8899aabbccddeeff),
                Limb(0x0011223344556677),
                Limb::ZERO,
                Limb::ZERO
            ]
        );
    }

    #[test]
    fn from_le_slice_too_long() {
        let bytes = hex!("ffeeddccbbaa99887766554433221100");
        assert_eq!(
            BoxedUint::from_be_slice(&bytes, 64),
            Err(DecodeError::InputSize)
        );
    }

    #[test]
    fn from_le_slice_not_word_sized() {
        let bytes = hex!("ffeeddccbbaa998877665544332211");
        assert_eq!(
            BoxedUint::from_be_slice(&bytes, 128),
            Err(DecodeError::InputSize)
        );
    }

    #[test]
    fn from_le_slice_bad_precision() {
        let bytes = hex!("ffeeddccbbaa99887766554433221100");
        assert_eq!(
            BoxedUint::from_le_slice(&bytes, 127),
            Err(DecodeError::Precision)
        );
    }
}