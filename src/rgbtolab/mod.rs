// Modified version of https://github.com/TooManyBees/lab

use lab::Lab;

// κ and ε parameters used in conversion between XYZ and La*b*.  See
// http://www.brucelindbloom.com/LContinuity.html for explanation as to why
// those are different values than those provided by CIE standard.
const KAPPA: f32 = 24389.0 / 27.0;
const EPSILON: f32 = 216.0 / 24389.0;

pub fn rgb_to_lab(rgb: &[f32; 3]) -> Lab {
    xyz_to_lab(rgb_to_xyz(rgb))
}

fn rgb_to_xyz(rgb: &[f32; 3]) -> [f32; 3] {
    let r = rgb_to_xyz_map(rgb[0]);
    let g = rgb_to_xyz_map(rgb[1]);
    let b = rgb_to_xyz_map(rgb[2]);

    [
        r * 0.4124564390896921 + g * 0.357576077643909 + b * 0.18043748326639894,
        r * 0.21267285140562248 + g * 0.715152155287818 + b * 0.07217499330655958,
        r * 0.019333895582329317 + g * 0.119192025881303 + b * 0.9503040785363677,
    ]
}

#[inline]
fn rgb_to_xyz_map(c: f32) -> f32 {
    if c > 10. / 255. {
        const A: f32 = 0.055;
        const D: f32 = 1.0 / 1.055;
        pow_2_4((c + A) * D)
    } else {
        const D: f32 = 1.0 / 12.92;
        c * D
    }
}

fn xyz_to_lab(xyz: [f32; 3]) -> Lab {
    let x = xyz_to_lab_map(xyz[0] * (1.0 / 0.95047));
    let y = xyz_to_lab_map(xyz[1]);
    let z = xyz_to_lab_map(xyz[2] * (1.0 / 1.08883));

    Lab {
        l: (116.0 * y) - 16.0,
        a: 500.0 * (x - y),
        b: 200.0 * (y - z),
    }
}

#[inline]
fn xyz_to_lab_map(c: f32) -> f32 {
    if c > EPSILON {
        c.powf(1.0 / 3.0)
    } else {
        (KAPPA * c + 16.0) * (1.0 / 116.0)
    }
}

fn pow_2_4(x: f32) -> f32 {
    // Closely approximate x^2.4.
    // Divide x by its exponent and a truncated version of itself to get it as close to 1 as
    // possible. Calculate the power of 2.4 using the binomial method. Multiply what was divided to
    // the power of 2.4.

    // Lookup tables still have to be hardcoded.
    const FRAC_BITS: u32 = 3;

    // Cast x into an integer to manipulate its exponent and fractional parts into indexes for
    // lookup tables.
    let bits = x.to_bits();

    // Get the integer log2 from the exponent part of bits
    let log2 = (bits >> 23) as i32 - 0x7f;

    // x is always >= (10/255 + A)*D so we only have to deal with a limited range in the exponent.
    // log2 range is [-4, 0]
    // Use a lookup table to offset for dividing by 2^log of x.
    // x^2.4 = (2^log2)^2.4 * (x/(2^log2))^2.4
    let lookup_entry_exp_pow_2_4 =
        |log2: i32| (f32::from_bits(((log2 + 0x7f) << 23) as u32) as f64).powf(2.4) as f32;
    let lookup_table_exp_pow_2_4 = [
        lookup_entry_exp_pow_2_4(-4),
        lookup_entry_exp_pow_2_4(-3),
        lookup_entry_exp_pow_2_4(-2),
        lookup_entry_exp_pow_2_4(-1),
        lookup_entry_exp_pow_2_4(0),
        lookup_entry_exp_pow_2_4(1),
        lookup_entry_exp_pow_2_4(2),
        lookup_entry_exp_pow_2_4(3),
    ];
    let exp_pow_2_4 = lookup_table_exp_pow_2_4[(log2 + 4) as usize];

    // Zero the exponent of x or divide by 2^log.
    let x = f32::from_bits((bits & 0x807fffff) | 0x3f800000);

    // Use lookup tables to divide by a truncated version of x and get an offset for that division.
    // x^2.4 = a^2.4 * (x/a)^2.4
    let lookup_entry_inv_truncated = |fraction: i32| {
        let truncated = 1.0 + (fraction as f64 + 0.5) / ((1 << FRAC_BITS) as f64);
        (1.0 / truncated) as f32
    };
    let lookup_table_inv_truncated = [
        lookup_entry_inv_truncated(0),
        lookup_entry_inv_truncated(1),
        lookup_entry_inv_truncated(2),
        lookup_entry_inv_truncated(3),
        lookup_entry_inv_truncated(4),
        lookup_entry_inv_truncated(5),
        lookup_entry_inv_truncated(6),
        lookup_entry_inv_truncated(7),
    ];
    let lookup_entry_truncated_pow_2_4 =
        |fraction: i32| (lookup_entry_inv_truncated(fraction) as f64).powf(-2.4) as f32;
    let lookup_table_truncated_pow_2_4 = [
        lookup_entry_truncated_pow_2_4(0),
        lookup_entry_truncated_pow_2_4(1),
        lookup_entry_truncated_pow_2_4(2),
        lookup_entry_truncated_pow_2_4(3),
        lookup_entry_truncated_pow_2_4(4),
        lookup_entry_truncated_pow_2_4(5),
        lookup_entry_truncated_pow_2_4(6),
        lookup_entry_truncated_pow_2_4(7),
    ];

    // Expose only FRAC_BITS of the fraction.
    let fraction = (bits >> (23 - FRAC_BITS) & ((1 << FRAC_BITS) - 1)) as usize;
    let truncated_pow_2_4 = lookup_table_truncated_pow_2_4[fraction];
    let x = x * lookup_table_inv_truncated[fraction];

    // Binomial series
    // Greater than 12 bits of precision.
    //let est = 7. / 25. - 24. / 25. * x + 42. / 25. * x.powi(2);
    // Plenty of precision.
    let est = 7. / 125. - 36. / 125. * x + 126. / 125. * x.powi(2) + 28. / 125. * x.powi(3);

    est * (truncated_pow_2_4 * exp_pow_2_4)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use self::avx2::*;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod avx2 {
    use super::*;

    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    macro_rules! sum_mult_avx {
        (($init:expr), $(($vec:expr, $mul:expr)),* ) => {
            {
                let mut sum = _mm256_set1_ps($init);
                $(
                    sum = _mm256_add_ps(sum, _mm256_mul_ps($vec, _mm256_set1_ps($mul)));
                )*
                sum
            }
        };
        ( $(($vec:expr, $mul:expr)),* ) => {
            sum_mult_avx!((0.0), $(($vec, $mul)),*);
        };
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn rgb_to_lab_avx2(rgb: &[__m256; 3]) -> [Lab; 8] {
        //xyz_to_lab_avx2(rgb_to_xyz_avx2(rgb))
        let xyz = rgb_to_xyz_avx2(rgb);
        #[target_feature(enable = "avx2")]
        unsafe fn to_array(reg: __m256) -> [f32; 8] {
            std::mem::transmute(reg)
        }
        let x = to_array(xyz[0]);
        let y = to_array(xyz[1]);
        let z = to_array(xyz[2]);

        let mut output = [Lab {
            l: 0.,
            a: 0.,
            b: 0.,
        }; 8];
        for i in 0..8 {
            output[i] = xyz_to_lab([x[i], y[i], z[i]]);
        }
        output
    }

    #[target_feature(enable = "avx2")]
    unsafe fn rgb_to_xyz_avx2(rgb: &[__m256; 3]) -> [__m256; 3] {
        let r = rgb_to_xyz_map_avx2(rgb[0]);
        let g = rgb_to_xyz_map_avx2(rgb[1]);
        let b = rgb_to_xyz_map_avx2(rgb[2]);

        let x = sum_mult_avx!(
            (r, 0.4124564390896921),
            (g, 0.357576077643909),
            (b, 0.18043748326639894)
        );
        let y = sum_mult_avx!(
            (r, 0.21267285140562248),
            (g, 0.715152155287818),
            (b, 0.07217499330655958)
        );
        let z = sum_mult_avx!(
            (r, 0.019333895582329317),
            (g, 0.119192025881303),
            (b, 0.9503040785363677)
        );

        [x, y, z]
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn rgb_to_xyz_map_avx2(c: __m256) -> __m256 {
        let low = _mm256_mul_ps(c, _mm256_set1_ps(1.0 / 12.92));
        let hi = pow_2_4_avx2(_mm256_mul_ps(
            _mm256_add_ps(c, _mm256_set1_ps(0.055)),
            _mm256_set1_ps(1.0 / 1.055),
        ));
        let select = _mm256_cmp_ps(c, _mm256_set1_ps(10. / 255.), _CMP_GT_OS);
        _mm256_blendv_ps(low, hi, select)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn pow_2_4_avx2(x: __m256) -> __m256 {
        // See non-avx2 version

        const FRAC_BITS: u32 = 3;

        let bits = _mm256_castps_si256(x);

        let log2_index =
            _mm256_add_epi32(_mm256_srli_epi32(bits, 23), _mm256_set1_epi32(-0x7f + 4));

        let lookup_entry_exp_pow_2_4 =
            |log2: i32| (f32::from_bits(((log2 + 0x7f) << 23) as u32) as f64).powf(2.4) as f32;
        let lookup_table_exp_pow_2_4 = _mm256_setr_ps(
            lookup_entry_exp_pow_2_4(-4),
            lookup_entry_exp_pow_2_4(-3),
            lookup_entry_exp_pow_2_4(-2),
            lookup_entry_exp_pow_2_4(-1),
            lookup_entry_exp_pow_2_4(0),
            lookup_entry_exp_pow_2_4(1),
            lookup_entry_exp_pow_2_4(2),
            lookup_entry_exp_pow_2_4(3),
        );

        let exp_pow_2_4 = _mm256_permutevar8x32_ps(lookup_table_exp_pow_2_4, log2_index);

        let x = _mm256_or_ps(
            _mm256_and_ps(
                x,
                _mm256_castsi256_ps(_mm256_set1_epi32(0x807fffffu32 as i32)),
            ),
            _mm256_castsi256_ps(_mm256_set1_epi32(0x3f800000)),
        );

        let lookup_entry_inv_truncated = |fraction: i32| {
            let truncated = 1.0 + (fraction as f64 + 0.5) / ((1 << FRAC_BITS) as f64);
            (1.0 / truncated) as f32
        };
        let lookup_table_inv_truncated = _mm256_setr_ps(
            lookup_entry_inv_truncated(0),
            lookup_entry_inv_truncated(1),
            lookup_entry_inv_truncated(2),
            lookup_entry_inv_truncated(3),
            lookup_entry_inv_truncated(4),
            lookup_entry_inv_truncated(5),
            lookup_entry_inv_truncated(6),
            lookup_entry_inv_truncated(7),
        );
        let lookup_entry_truncated_pow_2_4 =
            |fraction: i32| (lookup_entry_inv_truncated(fraction) as f64).powf(-2.4) as f32;
        let lookup_table_truncated_pow_2_4 = _mm256_setr_ps(
            lookup_entry_truncated_pow_2_4(0),
            lookup_entry_truncated_pow_2_4(1),
            lookup_entry_truncated_pow_2_4(2),
            lookup_entry_truncated_pow_2_4(3),
            lookup_entry_truncated_pow_2_4(4),
            lookup_entry_truncated_pow_2_4(5),
            lookup_entry_truncated_pow_2_4(6),
            lookup_entry_truncated_pow_2_4(7),
        );

        // No reason to mask the higher bits
        let fraction = _mm256_srli_epi32(bits, 23 - FRAC_BITS as i32);
        let truncated_pow_2_4 = _mm256_permutevar8x32_ps(lookup_table_truncated_pow_2_4, fraction);
        let x = _mm256_mul_ps(
            x,
            _mm256_permutevar8x32_ps(lookup_table_inv_truncated, fraction),
        );

        let x2 = _mm256_mul_ps(x, x);
        let x3 = _mm256_mul_ps(x2, x);
        let est = sum_mult_avx!(
            (7.0 / 125.0),
            (x, -36. / 125.),
            (x2, 126. / 125.),
            (x3, 28. / 125.)
        );

        _mm256_mul_ps(est, _mm256_mul_ps(truncated_pow_2_4, exp_pow_2_4))
    }
}
