#[cfg(feature = "halo2-pse")]
use crate::halo2_proofs::arithmetic::CurveAffine;
use crate::halo2_proofs::{arithmetic::FieldExt, circuit::Value};
use core::hash::Hash;
use num_bigint::BigInt;
use num_bigint::BigUint;
use num_bigint::Sign;
use num_traits::Signed;
use num_traits::{One, Zero};

/// Helper trait to convert to and from a [BigPrimeField] by converting a list of [u64] digits
#[cfg(feature = "halo2-axiom")]
pub trait BigPrimeField: ScalarField {
    /// Converts a slice of [u64] to [BigPrimeField]
    /// * `val`: the slice of u64
    /// Assumes val.len() <= 4
    fn from_u64_digits(val: &[u64]) -> Self;
}
#[cfg(feature = "halo2-axiom")]
impl<F> BigPrimeField for F
where
    F: FieldExt + Hash + Into<[u64; 4]> + From<[u64; 4]>,
{
    #[inline(always)]
    fn from_u64_digits(val: &[u64]) -> Self {
        debug_assert!(val.len() <= 4);
        let mut raw = [0u64; 4];
        raw[..val.len()].copy_from_slice(val);
        Self::from(raw)
    }
}

/// Helper trait to convert to and from a [ScalarField] by decomposing its an field element into [u64] limbs.
/// 
/// Note: Since the number of bits necessary to represent a field element is larger than the number of bits in a u64, we decompose the bit representation of the field element into multiple [u64] values e.g. `limbs`.
#[cfg(feature = "halo2-axiom")]
pub trait ScalarField: FieldExt + Hash {
    /// Returns the base `2<sup>bit_len</sup>` little endian representation of the [ScalarField] element up to `num_limbs` number of limbs (truncates any extra limbs).
    ///
    /// Assumes `bit_len < 64`.
    /// * `num_limbs`: number of limbs to return
    /// * `bit_len`: number of bits in each limb
    fn to_u64_limbs(self, num_limbs: usize, bit_len: usize) -> Vec<u64>;
}
#[cfg(feature = "halo2-axiom")]
impl<F> ScalarField for F
where
    F: FieldExt + Hash + Into<[u64; 4]>,
{
    #[inline(always)]
    fn to_u64_limbs(self, num_limbs: usize, bit_len: usize) -> Vec<u64> {
        // Basically same as `to_repr` but does not go further into bytes
        let tmp: [u64; 4] = self.into();
        decompose_u64_digits_to_limbs(tmp, num_limbs, bit_len)
    }
}

// Later: will need to separate BigPrimeField from ScalarField when Goldilocks is introduced

#[cfg(feature = "halo2-pse")]
pub trait BigPrimeField = FieldExt<Repr = [u8; 32]> + Hash;

#[cfg(feature = "halo2-pse")]
pub trait ScalarField = FieldExt + Hash;

/// Converts an [Iterator] of u64 digits into `number_of_limbs` limbs of `bit_len` bits returned as a [Vec].
///
/// Assumes: `bit_len < 64`.
/// * `e`: Iterator of [u64] digits
/// * `number_of_limbs`: number of limbs to return
/// * `bit_len`: number of bits in each limb
#[inline(always)]
pub(crate) fn decompose_u64_digits_to_limbs(
    e: impl IntoIterator<Item = u64>,
    number_of_limbs: usize,
    bit_len: usize,
) -> Vec<u64> {
    debug_assert!(bit_len < 64);

    let mut e = e.into_iter();
    // Mask to extract the bits from each digit
    let mask: u64 = (1u64 << bit_len) - 1u64;
    let mut u64_digit = e.next().unwrap_or(0);
    let mut rem = 64;

    // For each digit, we extract its individual limbs by repeatedly masking and shifting the digit based on how many bits we have left to extract.
    (0..number_of_limbs)
        .map(|_| match rem.cmp(&bit_len) {
            // If `rem` > `bit_len`, we mask the bits from the `u64_digit` to return the first limb.
            // We shift the digit to the right by `bit_len` bits and subtract `bit_len` from `rem`
            core::cmp::Ordering::Greater => {
                let limb = u64_digit & mask;
                u64_digit >>= bit_len;
                rem -= bit_len;
                limb
            }
            // If `rem` == `bit_len`, then we mask the bits from the `u64_digit` to return the first limb
            // We retrieve the next digit and reset `rem` to 64
            core::cmp::Ordering::Equal => {
                let limb = u64_digit & mask;
                u64_digit = e.next().unwrap_or(0);
                rem = 64;
                limb
            }
            // If `rem` < `bit_len`, we retrieve the next digit, mask it, and shift left `rem` bits from the `u64_digit` to return the first limb.
            // we shift the digit to the right by `bit_len` - `rem` bits to retrieve the start of the next limb and add 64 - bit_len to `rem` to get the remainder.
            core::cmp::Ordering::Less => {
                let mut limb = u64_digit;
                u64_digit = e.next().unwrap_or(0);
                limb |= (u64_digit & ((1 << (bit_len - rem)) - 1)) << rem;
                u64_digit >>= bit_len - rem;
                rem += 64 - bit_len;
                limb
            }
        })
        .collect()
}

/// Returns the number of bits needed to represent the value of `x`.
pub fn bit_length(x: u64) -> usize {
    (u64::BITS - x.leading_zeros()) as usize
}

/// Returns the ceiling of the base 2 logarithm of `x`.
/// 
/// Assumes x != 0
pub fn log2_ceil(x: u64) -> usize {
    (u64::BITS - x.leading_zeros() - (x & (x - 1) == 0) as u32) as usize
}

/// Returns the modulus of [BigPrimeField].
pub fn modulus<F: BigPrimeField>() -> BigUint {
    fe_to_biguint(&-F::one()) + 1u64
}

/// Returns the [BigPrimeField] element of 2<sup>n</sup>.
/// * `n`: the desired power of 2.
pub fn power_of_two<F: BigPrimeField>(n: usize) -> F {
    biguint_to_fe(&(BigUint::one() << n))
}

/// Converts an immutable reference to [BigUint] to a [BigPrimeField].
/// * `e`: immutable reference to [BigUint]
pub fn biguint_to_fe<F: BigPrimeField>(e: &BigUint) -> F {
    #[cfg(feature = "halo2-axiom")]
    {
        F::from_u64_digits(&e.to_u64_digits())
    }

    #[cfg(feature = "halo2-pse")]
    {
        let mut repr = F::Repr::default();
        let bytes = e.to_bytes_le();
        repr.as_mut()[..bytes.len()].copy_from_slice(&bytes);
        F::from_repr(repr).unwrap()
    }
}

/// Converts an immutable reference to [BigInt] to a [BigPrimeField].
/// * `e`: immutable reference to [BigInt]
pub fn bigint_to_fe<F: BigPrimeField>(e: &BigInt) -> F {
    #[cfg(feature = "halo2-axiom")]
    {
        let (sign, digits) = e.to_u64_digits();
        if sign == Sign::Minus {
            -F::from_u64_digits(&digits)
        } else {
            F::from_u64_digits(&digits)
        }
    }
    #[cfg(feature = "halo2-pse")]
    {
        let (sign, bytes) = e.to_bytes_le();
        let mut repr = F::Repr::default();
        repr.as_mut()[..bytes.len()].copy_from_slice(&bytes);
        let f_abs = F::from_repr(repr).unwrap();
        if sign == Sign::Minus {
            -f_abs
        } else {
            f_abs
        }
    }
}

/// Converts an immutable reference to an PrimeField element into a [BigUint] element. 
/// * `fe`: immutable reference to PrimeField element to convert
pub fn fe_to_biguint<F: ff::PrimeField>(fe: &F) -> BigUint {
    BigUint::from_bytes_le(fe.to_repr().as_ref())
}

/// Converts an immutable reference to a [BigPrimeField] element into a [BigInt] element. 
/// * `fe`: immutable reference to [BigPrimeField] element to convert
pub fn fe_to_bigint<F: BigPrimeField>(fe: &F) -> BigInt {
    // TODO: `F` should just have modulus as lazy_static or something
    let modulus = modulus::<F>();
    let e = fe_to_biguint(fe);
    if e <= &modulus / 2u32 {
        BigInt::from_biguint(Sign::Plus, e)
    } else {
        BigInt::from_biguint(Sign::Minus, modulus - e)
    }
}

/// Decomposes an immutable reference to a [BigPrimeField] element into `number_of_limbs` limbs of `bit_len` bits each and returns a [Vec] of [BigPrimeField] represented by those limbs.
/// * `e`: immutable reference to [BigPrimeField] element to decompose
/// * `number_of_limbs`: number of limbs to decompose `e` into
/// * `bit_len`: number of bits in each limb
pub fn decompose<F: BigPrimeField>(e: &F, number_of_limbs: usize, bit_len: usize) -> Vec<F> {
    if bit_len > 64 {
        decompose_biguint(&fe_to_biguint(e), number_of_limbs, bit_len)
    } else {
        decompose_fe_to_u64_limbs(e, number_of_limbs, bit_len).into_iter().map(F::from).collect()
    }
}

/// Decomposes an immutable reference to a [ScalarField] element into `number_of_limbs` limbs of `bit_len` bits each and returns a [Vec] of [u64] represented by those limbs.
///
/// Assumes `bit_len` < 64
/// * `e`: immutable reference to [ScalarField] element to decompose
/// * `number_of_limbs`: number of limbs to decompose `e` into
/// * `bit_len`: number of bits in each limb
pub fn decompose_fe_to_u64_limbs<F: ScalarField>(
    e: &F,
    number_of_limbs: usize,
    bit_len: usize,
) -> Vec<u64> {
    #[cfg(feature = "halo2-axiom")]
    {
        e.to_u64_limbs(number_of_limbs, bit_len)
    }

    #[cfg(feature = "halo2-pse")]
    {
        decompose_u64_digits_to_limbs(fe_to_biguint(e).iter_u64_digits(), number_of_limbs, bit_len)
    }
}

/// Decomposes an immutable reference to a [BigUint] into `num_limbs` limbs of `bit_len` bits each and returns a [Vec] of [BigPrimeField] represented by those limbs.
///
/// Assumes 64 <= `bit_len` < 128.
/// * `e`: immutable reference to [BigInt] to decompose
/// * `num_limbs`: number of limbs to decompose `e` into
/// * `bit_len`: number of bits in each limb
pub fn decompose_biguint<F: BigPrimeField>(
    e: &BigUint,
    num_limbs: usize,
    bit_len: usize,
) -> Vec<F> {
    // bit_len must be between 64` and 128
    debug_assert!((64..128).contains(&bit_len));
    let mut e = e.iter_u64_digits();

    // Grab first 128-bit limb from iterator
    let mut limb0 = e.next().unwrap_or(0) as u128;
    let mut rem = bit_len - 64;
    let mut u64_digit = e.next().unwrap_or(0);
    // Extract second limb (bit length 64) from e
    limb0 |= ((u64_digit & ((1 << rem) - 1u64)) as u128) << 64u32;
    u64_digit >>= rem;
    rem = 64 - rem;

    // Convert `limb0` into field element `F` and create an iterator by chaining `limb0` with the computing the remaining limbs
    core::iter::once(F::from_u128(limb0))
        .chain((1..num_limbs).map(|_| {
            let mut limb = u64_digit as u128;
            let mut bits = rem;
            u64_digit = e.next().unwrap_or(0);
            if bit_len >= 64 + bits {
                limb |= (u64_digit as u128) << bits;
                u64_digit = e.next().unwrap_or(0);
                bits += 64;
            }
            rem = bit_len - bits;
            limb |= ((u64_digit & ((1 << rem) - 1)) as u128) << bits;
            u64_digit >>= rem;
            rem = 64 - rem;
            F::from_u128(limb)
        }))
        .collect()
}

/// Decomposes an immutable reference to a [BigInt] into `num_limbs` limbs of `bit_len` bits each and returns a [Vec] of [BigPrimeField] represented by those limbs.
/// * `e`: immutable reference to `BigInt` to decompose
/// * `num_limbs`: number of limbs to decompose `e` into
/// * `bit_len`: number of bits in each limb
pub fn decompose_bigint<F: BigPrimeField>(e: &BigInt, num_limbs: usize, bit_len: usize) -> Vec<F> {
    if e.is_negative() {
        decompose_biguint::<F>(e.magnitude(), num_limbs, bit_len).into_iter().map(|x| -x).collect()
    } else {
        decompose_biguint(e.magnitude(), num_limbs, bit_len)
    }
}

/// Decomposes an immutable reference to a [BigInt] into `num_limbs` limbs of `bit_len` bits each and returns a [Vec] of [BigPrimeField] represented by those limbs wrapped in [Value].
/// 
/// Assumes `bit_len` < 128.
/// * `e`: immutable reference to `BigInt` to decompose
/// * `num_limbs`: number of limbs to decompose `e` into
/// * `bit_len`: number of bits in each limb
pub fn decompose_bigint_option<F: BigPrimeField>(
    value: Value<&BigInt>,
    number_of_limbs: usize,
    bit_len: usize,
) -> Vec<Value<F>> {
    value.map(|e| decompose_bigint(e, number_of_limbs, bit_len)).transpose_vec(number_of_limbs)
}

/// Wraps the internal value of `value` in an [Option]. 
/// If the value is [None], then the function returns [None].
/// * `value`: Value to convert.
pub fn value_to_option<V>(value: Value<V>) -> Option<V> {
    let mut v = None;
    value.map(|val| {
        v = Some(val);
    });
    v
}

/// Computes the value of an integer by passing as `input` a [Vec] of its limb values and the `bit_len` (bit length) used.
///
/// Returns the sum of all limbs scaled by 2<sup>(bit_len * i)</sup> where i is the index of the limb.
/// * `input`: Limb values of the integer.
/// * `bit_len`: Length of limb in bits
pub fn compose(input: Vec<BigUint>, bit_len: usize) -> BigUint {
    input.iter().rev().fold(BigUint::zero(), |acc, val| (acc << bit_len) + val)
}

#[cfg(feature = "halo2-axiom")]
pub use halo2_proofs_axiom::halo2curves::CurveAffineExt;

#[cfg(feature = "halo2-pse")]
pub trait CurveAffineExt: CurveAffine {
    /// Unlike the `Coordinates` trait, this just returns the raw affine (X, Y) coordinantes without checking `is_on_curve`
    fn into_coordinates(self) -> (Self::Base, Self::Base) {
        let coordinates = self.coordinates().unwrap();
        (*coordinates.x(), *coordinates.y())
    }
}
#[cfg(feature = "halo2-pse")]
impl<C: CurveAffine> CurveAffineExt for C {}

/// Module for reading parameters for Halo2 proving system from the file system.
pub mod fs {
    use std::{
        env::var,
        fs::{self, File},
        io::{BufReader, BufWriter},
    };

    use crate::halo2_proofs::{
        halo2curves::{
            bn256::{Bn256, G1Affine},
            CurveAffine,
        },
        poly::{
            commitment::{Params, ParamsProver},
            kzg::commitment::ParamsKZG,
        },
    };
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

    /// Reads the srs from a file found in `./params/kzg_bn254_{k}.srs` or `{dir}/kzg_bn254_{k}.srs` if `PARAMS_DIR` env var is specified.
    /// * `k`: degree that expresses the size of circuit (i.e., 2^<sup>k</sup> is the number of rows in the circuit)
    pub fn read_params(k: u32) -> ParamsKZG<Bn256> {
        let dir = var("PARAMS_DIR").unwrap_or_else(|_| "./params".to_string());
        ParamsKZG::<Bn256>::read(&mut BufReader::new(
            File::open(format!("{dir}/kzg_bn254_{k}.srs").as_str())
                .expect("Params file does not exist"),
        ))
        .unwrap()
    }

    /// Attempts to read the srs from a file found in `./params/kzg_bn254_{k}.srs` or `{dir}/kzg_bn254_{k}.srs` if `PARAMS_DIR` env var is specified, creates a file it if it does not exist.
    /// * `k`: degree that expresses the size of circuit (i.e., 2^<sup>k</sup> is the number of rows in the circuit)
    /// * `setup`: a function that creates the srs
    pub fn read_or_create_srs<'a, C: CurveAffine, P: ParamsProver<'a, C>>(
        k: u32,
        setup: impl Fn(u32) -> P,
    ) -> P {
        let dir = var("PARAMS_DIR").unwrap_or_else(|_| "./params".to_string());
        let path = format!("{dir}/kzg_bn254_{k}.srs");
        match File::open(path.as_str()) {
            Ok(f) => {
                #[cfg(feature = "display")]
                println!("read params from {path}");
                let mut reader = BufReader::new(f);
                P::read(&mut reader).unwrap()
            }
            Err(_) => {
                #[cfg(feature = "display")]
                println!("creating params for {k}");
                fs::create_dir_all(dir).unwrap();
                let params = setup(k);
                params.write(&mut BufWriter::new(File::create(path).unwrap())).unwrap();
                params
            }
        }
    }

    /// Generates the SRS for the KZG scheme and writes it to a file found in "./params/{dir}/kzg_bn254_{k}.srs" 
    /// * `k`: degree that expresses the size of circuit (i.e., 2^<sup>k</sup> is the number of rows in the circuit)
    pub fn gen_srs(k: u32) -> ParamsKZG<Bn256> {
        read_or_create_srs::<G1Affine, _>(k, |k| {
            ParamsKZG::<Bn256>::setup(k, ChaCha20Rng::from_seed(Default::default()))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::halo2_proofs::halo2curves::bn256::Fr;
    use num_bigint::RandomBits;
    use rand::{rngs::OsRng, Rng};
    use std::ops::Shl;

    use super::*;

    #[test]
    fn test_signed_roundtrip() {
        use crate::halo2_proofs::halo2curves::bn256::Fr;
        assert_eq!(fe_to_bigint(&bigint_to_fe::<Fr>(&-BigInt::one())), -BigInt::one());
    }

    #[test]
    fn test_decompose_biguint() {
        let mut rng = OsRng;
        const MAX_LIMBS: u64 = 5;
        for bit_len in 64..128usize {
            for num_limbs in 1..=MAX_LIMBS {
                for _ in 0..10_000usize {
                    let mut e: BigUint = rng.sample(RandomBits::new(num_limbs * bit_len as u64));
                    let limbs = decompose_biguint::<Fr>(&e, num_limbs as usize, bit_len);

                    let limbs2 = {
                        let mut limbs = vec![];
                        let mask = BigUint::one().shl(bit_len) - 1usize;
                        for _ in 0..num_limbs {
                            let limb = &e & &mask;
                            let mut bytes_le = limb.to_bytes_le();
                            bytes_le.resize(32, 0u8);
                            limbs.push(Fr::from_bytes(&bytes_le.try_into().unwrap()).unwrap());
                            e >>= bit_len;
                        }
                        limbs
                    };
                    assert_eq!(limbs, limbs2);
                }
            }
        }
    }

    #[test]
    fn test_decompose_u64_digits_to_limbs() {
        let mut rng = OsRng;
        const MAX_LIMBS: u64 = 5;
        for bit_len in 0..64usize {
            for num_limbs in 1..=MAX_LIMBS {
                for _ in 0..10_000usize {
                    let mut e: BigUint = rng.sample(RandomBits::new(num_limbs * bit_len as u64));
                    let limbs = decompose_u64_digits_to_limbs(
                        e.to_u64_digits(),
                        num_limbs as usize,
                        bit_len,
                    );
                    let limbs2 = {
                        let mut limbs = vec![];
                        let mask = BigUint::one().shl(bit_len) - 1usize;
                        for _ in 0..num_limbs {
                            let limb = &e & &mask;
                            limbs.push(u64::try_from(limb).unwrap());
                            e >>= bit_len;
                        }
                        limbs
                    };
                    assert_eq!(limbs, limbs2);
                }
            }
        }
    }
}
