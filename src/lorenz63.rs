//! Lorenz three-variables system
//! https://en.wikipedia.org/wiki/Lorenz_system

use ndarray::*;

use crate::traits::*;

#[derive(Clone, Copy, Debug)]
pub struct Lorenz63 {
    pub p: f64,
    pub r: f64,
    pub b: f64,
}

impl Default for Lorenz63 {
    fn default() -> Self {
        Lorenz63 {
            p: 10.0,
            r: 28.0,
            b: 8.0 / 3.0,
        }
    }
}

impl Lorenz63 {
    pub fn new(p: f64, r: f64, b: f64) -> Self {
        Lorenz63 { p: p, r: r, b: b }
    }
}

impl ModelSpec for Lorenz63 {
    type Scalar = f64;
    type Dim = Ix1;

    fn model_size(&self) -> usize {
        3
    }
}

impl IdaModel for Lorenz63 {
    fn residual<'a, S>(&mut self, v: &'a mut ArrayBase<S, Ix1>) -> &'a mut ArrayBase<S, Ix1>
    where
        S: DataMut<Elem = Self::Scalar>,
    {
        let x = v[0];
        let y = v[1];
        let z = v[2];
        v[0] = self.p * (y - x);
        v[1] = x * (self.r - z) - y;
        v[2] = x * y - self.b * z;
        v
    }

    fn jacobian<S>(
        &mut self,
        cj: Self::Scalar,
        yy: &ArrayView<S, Self::Dim>,
        yp: &ArrayView<S, Self::Dim>,
    ) -> ()
    where
        S: DataMut<Elem = Self::Scalar>,
    {
        unimplemented!();
    }
}
