#![allow(dead_code)]
#![allow(unused_macros)]
#![macro_use]

use ndarray_rand::rand as rand;
use ndarray_rand::rand_distr as rand_distr;
use rand_distr::{Uniform,StandardNormal};
use rand::{SeedableRng,Rng,rngs::StdRng};
use crate::math::{Real,real};

pub struct Random {
    pub rng:StdRng,
    dist:Uniform<Real>
}

impl Random {
    pub fn integer(&mut self,n:usize)->usize {
	self.number(0.0,real(n)) as usize
    }

    pub fn number(&mut self,x0:Real,x1:Real)->Real {
        x0+(x1-x0)*self.rng.sample(self.dist)
    }

    pub fn normal(&mut self)->Real {
	self.rng.sample(StandardNormal)
    }

    pub fn new()->Self {
        let rng = SeedableRng::from_entropy();
        let dist : Uniform<Real> = Uniform::new(0.0,1.0);
        Random{ rng, dist }
    }
}
