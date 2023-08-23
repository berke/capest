#![allow(dead_code)]
#![allow(unused_macros)]
#![macro_use]

pub use ndarray::{array,s,ArrayBase,
	      Array1,Array2,Array3,Array4,
	      ArrayView1,ArrayView2,ArrayView3,ArrayView4,
	      ArrayViewMut1,ArrayViewMut2,ArrayViewMut3,ArrayViewMut4,
	      linalg::Dot};
pub use ndarray_linalg::norm::Norm;
pub use serde::{Serialize,Deserialize};
use std::fmt::{Display,Formatter};

pub fn floor<T:num::traits::real::Real>(x:T)->T { x.floor() }
pub fn ceil<T:num::traits::real::Real>(x:T)->T { x.ceil() }
pub fn abs<T:num::traits::real::Real>(x:T)->T { x.abs() }
pub fn exp<T:num::traits::real::Real>(x:T)->T { x.exp() }
pub fn log<T:num::traits::real::Real>(x:T)->T { x.ln() }
pub fn cos<T:num::traits::real::Real>(x:T)->T { x.cos() }
pub fn sin<T:num::traits::real::Real>(x:T)->T { x.sin() }
pub fn tan<T:num::traits::real::Real>(x:T)->T { x.tan() }
pub fn cosh<T:num::traits::real::Real>(x:T)->T { x.cosh() }
pub fn sinh<T:num::traits::real::Real>(x:T)->T { x.sinh() }
pub fn tanh<T:num::traits::real::Real>(x:T)->T { x.tanh() }
pub fn sech<T:num::traits::real::Real>(x:T)->T { T::one()/x.cosh() }
pub fn acos<T:num::traits::real::Real>(x:T)->T { x.acos() }
pub fn asin<T:num::traits::real::Real>(x:T)->T { x.asin() }
pub fn atan<T:num::traits::real::Real>(x:T)->T { x.atan() }
pub fn sqrt<T:num::traits::real::Real>(x:T)->T { x.sqrt() }
pub fn cbrt<T:num::traits::real::Real>(x:T)->T { x.cbrt() }
pub fn min<T:num::traits::real::Real>(x:T,y:T)->T { x.min(y) }
pub fn max<T:num::traits::real::Real>(x:T,y:T)->T { x.max(y) }
pub fn pow<T:num::traits::real::Real>(x:T,y:T)->T { x.powf(y) }
pub fn powi<T:num::traits::real::Real>(x:T,y:i32)->T { x.powi(y) }
pub fn sq<T:num::traits::real::Real>(x:T)->T { x*x }
pub fn cube<T:num::traits::real::Real>(x:T)->T { x*x*x }
pub fn hypot<T:num::traits::real::Real>(x:T,y:T)->T { x.hypot(y) }
pub fn modulo(x:isize,y:isize)->usize {
    let z=x%y;
    if z<0 { (z+y) as usize } else { z as usize }
}
pub fn sign<T:num::traits::real::Real>(x:T)->T { x.signum() }
pub const E:f64 = std::f64::consts::E;
pub const PI:f64 = std::f64::consts::PI;
pub const EPSILON:f64 = std::f64::EPSILON;
pub const DEGREE:f64 = PI/180.0;
pub const IMAG:Complex = Complex{ re:0.0, im:1.0 };
pub const INFINITY:f64 = std::f64::INFINITY;
pub const NAN:f64 = std::f64::NAN;

pub type AR1 = Array1::<Real>;
pub type AR2 = Array2::<Real>;
pub type AR3 = Array3::<Real>;
pub type AR4 = Array4::<Real>;
pub type AU1 = Array1::<usize>;
pub type AU2 = Array2::<usize>;
pub type AU3 = Array3::<usize>;
pub type AU4 = Array4::<usize>;
pub type AB1 = Array1::<bool>;
pub type AB2 = Array2::<bool>;
pub type AB3 = Array3::<bool>;
pub type AB4 = Array4::<bool>;

pub type Real = f64;
pub type Complex = num::Complex<Real>;
pub type V = Array1<Real>;
pub type M = Array2<Real>;
pub type A3 = Array3<Real>;
pub type A4 = Array4<Real>;

pub fn close_enough(x:f64,y:f64,tol:f64)->bool {
    let ax = abs(x);
    let ay = abs(y);
    let a = if ax>ay { ax } else { ay };
    let e = abs(x-y);
    let e = if a > tol { e/a } else { e };
    e < tol
}

macro_rules! assert_close {
    ($x:expr,$y:expr,$tol:expr) => {
        if !close_enough($x,$y,$tol) {
            println!("Tolerance failure: |{:.6e} - {:.6e}| @ {:.6e}",$x,$y,$tol);
            panic!("Tolerance failure");
        }
    }
}

pub trait Complexable {
    fn complex(self)->Complex;
}

impl Complexable for Real {
    fn complex(self)->Complex {
        Complex{ re:self, im:0.0 }
    }
}

pub fn complex<T:Complexable>(x:T)->Complex {
    x.complex()
}

pub trait Realable {
    fn real(&self)->Real;
}

impl Realable for u16 {
    fn real(&self)->Real {
        *self as Real
    }
}

impl Realable for u64 {
    fn real(&self)->Real {
        *self as Real
    }
}

impl Realable for usize {
    fn real(&self)->Real {
        *self as Real
    }
}

impl Realable for isize {
    fn real(&self)->Real {
        *self as Real
    }
}

pub fn real<T:Realable>(x:T)->Real {
    x.real()
}

use std::ops::{Add,Sub,Mul,Div,Neg,Index,IndexMut};

#[derive(Clone,Copy,Debug,Serialize,Deserialize)]
pub struct Real3(pub [Real;3]);

pub const R3X:Real3 = Real3([1.0,0.0,0.0]);
pub const R3Y:Real3 = Real3([0.0,1.0,0.0]);
pub const R3Z:Real3 = Real3([0.0,0.0,1.0]);

#[derive(Clone,Copy,Debug,Serialize,Deserialize)]
pub struct Real33(pub [Real;9]);

pub fn r3(x:Real,y:Real,z:Real)->Real3 { Real3::make([x,y,z]) }

impl Display for Real3 {
    fn fmt(&self,fmt:&mut Formatter)->Result<(),std::fmt::Error> {
	write!(fmt,"[{:+8.5e},{:+8.5e},{:+8.5e}]",
		self[0],
		self[1],
		self[2])
    }
}

impl Neg for Real3 {
    type Output = Real3;
    fn neg(self)->Self { let Real3([x,y,z]) = self; r3(-x,-y,-z) }
}

impl Add for Real3 {
    type Output = Real3;
    fn add(self,Real3([x2,y2,z2]):Self)->Self { let Real3([x1,y1,z1]) = self; r3(x1+x2,y1+y2,z1+z2) }
}

impl Sub for Real3 {
    type Output = Real3;
    fn sub(self,Real3([x2,y2,z2]):Self)->Self { let Real3([x1,y1,z1]) = self; r3(x1-x2,y1-y2,z1-z2) }
}

impl Mul<Real3> for Real {
    type Output = Real3;
    fn mul(self,Real3([x,y,z]):Real3)->Real3 { r3(self*x,self*y,self*z) }
}

impl Mul<&Real3> for Real {
    type Output = Real3;
    fn mul(self,Real3([x,y,z]):&Real3)->Real3 { r3(self*x,self*y,self*z) }
}

impl Div<Real> for Real3 {
    type Output = Real3;
    fn div(self,k:Real)->Real3 {
        let Real3([x,y,z]) = self;
        r3(x/k,y/k,z/k)
    }
}

impl Index<usize> for Real3 {
    type Output = Real;
    fn index(&self, i:usize) -> &Self::Output {
        let Real3(u) = self;
        &u[i]
    }
}

impl IndexMut<usize> for Real3 {
    fn index_mut(&mut self, i:usize) -> &mut Self::Output {
        let Real3(u) = self;
        &mut u[i]
    }
}

impl Real3 {
    pub fn zero()->Self { Real3([0.0;3]) }
    pub fn make(u:[Real;3])->Self { Real3(u) }
    pub fn dot(self,Real3([x2,y2,z2]):Self)->Real {
        let Real3([x1,y1,z1]) = self;
        x1*x2+y1*y2+z1*z2
    }
    pub fn norm2sq(self)->Real { self.dot(self) }
    pub fn norm2(self)->Real { sqrt(self.norm2sq()) }
    pub fn cross(self,Real3([x2,y2,z2]):Self)->Self {
        let Real3([x1,y1,z1]) = self;
        r3(y1*z2-y2*z1,x2*z1-x1*z2,x1*y2-x2*y1)
    }
    pub fn scale(self,k:Real)->Self {
        let Real3([x,y,z]) = self;
        r3(k*x,k*y,k*z)
    }
    pub fn rotate(self,axis:Self,theta:Real)->Self {
        cos(theta)*self + sin(theta)*axis.cross(self)+(1.0-cos(theta))*axis.dot(self)*axis
    }
    pub fn cosangle(self,y:Real3)->Real {
        let n1 = self.norm2();
        let n2 = y.norm2();
        if n1 < EPSILON || n2 < EPSILON {
            0.0
        } else {
            (self/n1).dot(y/n2) // XXX: Figure out if we can implement some traits to help with the
                                     // referencing/dereferencing
        }
    }
    pub fn angle(self,y:Real3)->Real {
        acos(self.cosangle(y))
    }
}

impl Index<[usize;2]> for Real33 {
    type Output = Real;
    fn index(&self, ij:[usize;2]) -> &Self::Output {
        let Real33(a) = self;
        let [i,j] = ij;
        assert!(i < 3 && j < 3);
        &a[3*i+j]
    }
}

impl IndexMut<[usize;2]> for Real33 {
    fn index_mut(&mut self, ij:[usize;2]) -> &mut Real {
        let Real33(a) = self;
        let [i,j] = ij;
        assert!(i < 3 && j < 3);
        &mut a[3*i+j]
    }
}

impl Real33 {
    pub fn zero()->Self { Real33([0.0;9]) }
    pub fn identity()->Self {
        let mut c = Self::zero();
        c[[0,0]] = 1.0;
        c[[1,1]] = 1.0;
        c[[2,2]] = 1.0;
        c
    }
    pub fn col(&self,j:usize)->Real3 { r3(self[[0,j]],self[[1,j]],self[[2,j]]) }
    pub fn gemv(&self,x:&Real3)->Real3 {
        let mut b = Real3::zero();
        for j in 0..3 {
            b = b + x[j]*self.col(j)
        }
        b
    }
    pub fn gemm(&self,_y:&Real33)->Self {
        panic!("Not implemented")
    }
    pub fn scale(&self,k:Real)->Self {
        let mut b = Self::zero();
        for i in 0..9 {
            b.0[i] = k*self.0[i]
        }
        b
    }
}

// impl Vector3 for Real3 {

#[cfg(test)]
#[test]
fn test_vector() {
    let x = Real3::make([-1.0,2.0,3.33]);
    let mut y = Real3::zero();
    y[0] = 1.2;
    y[1] = 3.4;
    y[2] = 4.5;
    let xpy = x + y;
    println!("xpy={:?}",xpy);
}
