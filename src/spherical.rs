use crate::math::*;
use crate::math_random::Random;

pub fn sample_spherical(rng:&mut Random)->Real3 {
    let rs1 = rng.number(0.0,1.0);
    let rs2 = rng.number(0.0,1.0);
    let theta = 2.0*PI*rs1;
    let phi = 2.0*asin(sqrt(rs2));
    let sp = sin(phi);
    Real3::make([sp*cos(theta),sp*sin(theta),cos(phi)])
}
