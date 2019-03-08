use failure::Fail;
use ndarray::*;

use crate::traits::{ModelSpec, Residual};

#[derive(Debug, Fail)]
pub enum Error {
    // Recoverable
    /// SUN_NLS_CONTINUE
    /// not converged, keep iterating
    #[fail(display = "")]
    Continue {},

    /// convergece failure, try to recover
    /// SUN_NLS_CONV_RECVR
    #[fail(display = "")]
    ConvergenceRecover {},

    // Unrecoverable
    /// illegal function input
    ///SUN_NLS_ILL_INPUT
    #[fail(display = "")]
    IllegalInput {},
    // failed NVector operation
    //SUN_NLS_VECTOROP_ERR
}

pub trait NLProblem: ModelSpec {
    /// `sys` evaluates the nonlinear system `F(y)` for ROOTFIND type modules or `G(y)` for
    /// FIXEDPOINT type modules.
    ///
    /// # Arguments
    ///
    /// * `y` is the state vector at which the nonlinear system should be evaluated.
    /// * `f` is the output vector containing `F(y)` or `G(y)`, depending on the solver type.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` indicates whether the routine has updated the Jacobian A (`true`) or not (`false`).
    /// * `Err()` for a recoverable error,
    /// * `Err(_) for an unrecoverable error
    fn sys<S1, S2>(
        &self,
        y: &ArrayBase<S1, Ix1>,
        f: &mut ArrayBase<S2, Ix1>,
    ) -> Result<(), failure::Error>
    where
        S1: Data<Elem = <Self as ModelSpec>::Scalar>,
        S2: DataMut<Elem = <Self as ModelSpec>::Scalar>;

    /// `lsetup` is called by integrators to provide the nonlinear solver with access to its linear
    /// solver setup function.
    ///
    /// # Arguments
    ///
    /// * `y` is the state vector at which the linear system should be setup.
    /// * `F` is the value of the nonlinear system function at y.
    /// * `jbad` is an input indicating whether the nonlinear solver believes that A has gone stale
    ///     (`true`) or not (`false`).
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` indicates whether the routine has updated the Jacobian A (`true`) or not
    ///     (`false`).
    /// * `Err()` for a recoverable error,
    /// * `Err(_) for an unrecoverable error
    ///
    /// The `lsetup` function sets up the linear system `Ax = b` where `A = ∂F/∂y` is the
    /// linearization of the nonlinear residual function `F(y) = 0` (when using direct linear
    /// solvers) or calls the user-defined preconditioner setup function (when using iterative
    /// linear solvers). `lsetup` implementations that do not require solving this system, do not
    /// utilize linear solvers, or use linear solvers that do not require setup may ignore these
    /// functions.
    fn lsetup<S1>(
        &mut self,
        y: &ArrayBase<S1, Ix1>,
        F: &ArrayView<<Self as ModelSpec>::Scalar, Ix1>,
        jbad: bool,
    ) -> Result<bool, failure::Error>
    where
        S1: Data<Elem = <Self as ModelSpec>::Scalar>,
    {
        Ok(false)
    }

    /// `lsolve` is called by integrators to provide the nonlinear solver with access to its linear
    /// solver solve function.
    ///
    /// # Arguments
    /// * `y` is the input vector containing the current nonlinear iteration.
    /// * `b` contains the right-hand side vector for the linear solve on input and the solution to
    ///     the linear system on output.
    ///
    /// # Returns
    ///
    /// Return value The return value retval (of type int) is zero for a successul solve, a positive value for a recoverable error, and a negative value for an unrecoverable error.
    ///
    /// # Notes
    ///
    /// The `lsove` function solves the linear system `Ax = b` where `A = ∂F/∂y` is the linearization
    /// of the nonlinear residual function F(y) = 0. Implementations that do not require solving
    /// this system or do not use sunlinsol linear solvers may ignore these functions.
    fn lsolve<S1, S2>(
        &self,
        y: &ArrayBase<S1, Ix1>,
        b: &mut ArrayBase<S2, Ix1>,
    ) -> Result<(), failure::Error>
    where
        S1: Data<Elem = <Self as ModelSpec>::Scalar>,
        S2: DataMut<Elem = <Self as ModelSpec>::Scalar>;

    /// `ctest` is an integrator-specific convergence test for nonlinear solvers and are typically
    /// supplied by each integrator, but users may supply custom problem-specific versions as desired.
    ///
    /// # Arguments
    ///
    /// * `y` is the current nonlinear iterate.
    /// * `del` is the difference between the current and prior nonlinear iterates.
    /// * `tol` is the nonlinear solver tolerance.
    /// * `ewt` is the weight vector used in computing weighted norms.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - the iteration is converged.
    /// * `Ok(false)` - the iteration has not converged, keep iterating.
    /// * `Err(Error::ConvergenceRecover)` - the iteration appears to be diverging, try to recover.
    /// * `Err(_)` - an unrecoverable error occurred.
    ///
    /// # Notes
    ///
    /// The tolerance passed to this routine by integrators is the tolerance in a weighted
    /// root-mean-squared norm with error weight vector `ewt`. Modules utilizing their own
    /// convergence criteria may ignore these functions.
    fn ctest<S1, S2, S3>(
        &self,
        y: &ArrayBase<S1, Ix1>,
        del: &ArrayBase<S2, Ix1>,
        tol: <Self as ModelSpec>::Scalar,
        ewt: &ArrayBase<S3, Ix1>,
    ) -> Result<bool, failure::Error>
    where
        S1: Data<Elem = <Self as ModelSpec>::Scalar>,
        S2: Data<Elem = <Self as ModelSpec>::Scalar>,
        S3: Data<Elem = <Self as ModelSpec>::Scalar>;
}

pub trait NLSolver<P: NLProblem> {
    /// Solves the nonlinear system `F(y)=0` or `G(y)=y`.
    ///
    /// # Arguments
    ///
    /// * `problem` -
    /// * `y0` - the initial iterate for the nonlinear solve.
    /// * `y` - (output) the solution to the nonlinear system.
    /// * `w` - the solution error weight vector used for computing weighted error norms.
    /// * `tol` - the requested solution tolerance in the weighted root-mean-squared norm.
    /// * `call_lsetup` - a flag indicating that the integrator recommends for the linear solver
    ///     setup function to be called.
    ///
    /// Note: The `lsetup` function sets up the linear system `Ax = b` where `A = ∂F/∂y` is the
    /// linearization of the nonlinear residual function `F(y) = 0` (when using direct linear
    /// solvers) or calls the user-defined preconditioner setup function (when using iterative
    /// linear solvers). Implementations that do not require solving this system, do not utilize
    /// linear solvers, or use linear solvers that do not require setup may skip the implementation.
    ///
    /// # Returns
    ///
    /// * Ok(()) - Successfully converged on a solution
    ///
    /// # Errors
    ///
    /// * `Err(Error::ConvergenceRecover)` - the iteration appears to be diverging, try to recover.
    /// * `Err(_)` - an unrecoverable error occurred.
    fn solve<S1, S2>(
        &mut self,
        problem: &mut P,
        y0: &ArrayBase<S1, Ix1>,
        y: &mut ArrayBase<S2, Ix1>,
        w: &ArrayBase<S1, Ix1>,
        tol: P::Scalar,
        call_lsetup: bool,
    ) -> Result<(), failure::Error>
    where
        S1: Data<Elem = P::Scalar>,
        S2: DataMut<Elem = P::Scalar>;
}