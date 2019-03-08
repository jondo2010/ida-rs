use ndarray::*;

use crate::nonlinear::traits::*;
use crate::traits::ModelSpec;

#[derive(Debug)]
struct Newton<P: NLProblem> {
    /// Newton update vector
    delta: Array1<P::Scalar>,
    /// Jacobian status, current = `true` / stale = `false`
    jcur: bool,
    /// current number of iterations in a solve attempt
    curiter: usize,
    /// maximum number of iterations in a solve attempt
    maxiters: usize,
    /// total number of nonlinear iterations across all solves
    niters: usize,
    /// total number of convergence failures across all solves
    nconvfails: usize,
}

impl<P: NLProblem> Newton<P> {
    pub fn new(size: usize, maxiters: usize) -> Self {
        Newton {
            delta: Array::zeros(size),
            jcur: false,
            curiter: 0,
            maxiters: maxiters,
            niters: 0,
            nconvfails: 0,
        }
    }
}

impl<P> NLSolver<P> for Newton<P>
where
    P: NLProblem,
    P::Scalar: num_traits::Float + num_traits::NumRef + num_traits::NumAssignRef + std::fmt::Debug,
{
    ///
    /// # Arguments
    ///
    /// * `problem` -
    /// * `y0` -
    /// * `y` -
    /// * `w` -
    /// * `tol` -
    /// * `call_lsetup` -
    ///
    /// Note: The SUNNonlinLSetupFn function sets up the linear system `Ax = b` where `A = ∂F/∂y` is
    /// the linearization of the nonlinear residual function `F(y) = 0` (when using sunlinsol direct
    /// linear solvers) or calls the user-defined preconditioner setup function (when using
    /// sunlinsol iterative linear solvers). sunnonlinsol implementations that do not require
    /// solving this system, do not utilize sunlinsol linear solvers, or use sunlinsol linear
    /// solvers that do not require setup may set this operation to NULL.
    ///
    /// Performs the nonlinear solve `F(y) = 0`
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
        S2: DataMut<Elem = P::Scalar>,
    {
        // assume the Jacobian is good
        let mut jbad = false;

        // looping point for attempts at solution of the nonlinear system: Evaluate the nonlinear
        // residual function (store in delta) Setup the linear solver if necessary Preform Newton
        // iteraion
        let retval: Result<(), failure::Error> = 'outer: loop {
            // compute the nonlinear residual, store in delta
            let retval = problem
                .sys(y0, &mut self.delta)
                .and_then(|_| {
                    // if indicated, setup the linear system
                    if call_lsetup {
                        problem
                            .lsetup(y0, &self.delta.view(), jbad)
                            .map(|jcur| self.jcur = jcur)
                    } else {
                        Ok(())
                    }
                })
                .and_then(|_| {
                    // initialize counter curiter
                    self.curiter = 0;
                    // load prediction into y
                    y.assign(&y0);
                    // looping point for Newton iteration. Break out on any error.
                    'inner: loop {
                        // increment nonlinear solver iteration counter
                        self.niters += 1;
                        // compute the negative of the residual for the linear system rhs
                        self.delta.mapv_inplace(P::Scalar::neg);
                        // solve the linear system to get Newton update delta
                        let retval =
                            NLProblem::lsolve(problem, y, &mut self.delta).and_then(|_| {
                                // update the Newton iterate
                                *y += &self.delta;
                                // test for convergence
                                NLProblem::ctest(problem, y, &self.delta.view(), tol, w).and_then(
                                    |converged| {
                                        if converged {
                                            // if successful update Jacobian status and return
                                            self.jcur = false;
                                            Ok(true)
                                        } else {
                                            self.curiter += 1;
                                            if self.curiter >= self.maxiters {
                                                Err(failure::Error::from(
                                                    Error::ConvergenceRecover {},
                                                ))
                                            } else {
                                                // compute the nonlinear residual, store in delta
                                                // Ok(false) will continue to iterate 'inner
                                                problem.sys(y, &mut self.delta).and(Ok(false))
                                            }
                                        }
                                    },
                                )
                            });

                        // check if the iteration should continue; otherwise exit Newton loop
                        if let Ok(false) = retval {
                            continue 'inner;
                        } else {
                            break retval.and(Ok(()));
                        }
                    } // end of Newton iteration loop
                });

            // all inner-loop results go here

            match &retval {
                Ok(_) => {
                    return retval;
                }

                Err(error) => {
                    // If there is a recoverable convergence failure and the Jacobian-related data
                    // appears not to be current, increment the convergence failure count and loop
                    // again with a call to lsetup in which jbad = true.
                    if let Some(Error::ConvergenceRecover {}) = error.downcast_ref::<Error>() {
                        if !self.jcur {
                            self.nconvfails += 1;
                            call_lsetup = true;
                            jbad = true;
                            continue 'outer;
                        }
                    }
                }
            }
            // Otherwise break out and return.
            break 'outer retval;
        }; // end of setup loop

        // increment number of convergence failures
        self.nconvfails += 1;

        // all error returns exit here
        retval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonlinear::traits::*;
    use crate::traits::ModelSpec;
    use ndarray::*;
    use nearly_eq::assert_nearly_eq;

    #[derive(Clone, Debug)]
    struct TestProblem {
        A: Array<f64, Ix2>,
        x: Array<f64, Ix1>,
    }

    impl ModelSpec for TestProblem {
        type Scalar = f64;
        type Dim = Ix1;

        fn model_size(&self) -> usize {
            3
        }
    }

    impl TestProblem {
        /// Jacobian of the nonlinear residual function
        ///
        ///            ( 2x  2y  2z )
        /// J(x,y,z) = ( 4x  2y  -4 )
        ///            ( 6x  -4  2z )  
        fn jac<S1, S2, S3>(
            _t: f64,
            y: &ArrayBase<S1, Ix1>,
            _fy: &ArrayBase<S2, Ix1>,
            j: &mut ArrayBase<S3, Ix2>,
        ) -> Result<(), failure::Error>
        where
            S1: Data<Elem = f64>,
            S2: Data<Elem = f64>,
            S3: DataMut<Elem = f64>,
        {
            j.assign(&array![
                [2.0 * y[0], 2.0 * y[1], 2.0 * y[2]],
                [4.0 * y[0], 2.0 * y[1], -4.0],
                [6.0 * y[0], -4.0, 2.0 * y[2]]
            ]);
            Ok(())
        }
    }

    impl NLProblem for TestProblem {
        /// Nonlinear residual function
        ///
        /// f1(x,y,z) = x^2 + y^2 + z^2 - 1 = 0
        /// f2(x,y,z) = 2x^2 + y^2 - 4z     = 0
        /// f3(x,y,z) = 3x^2 - 4y + z^2     = 0
        fn sys<S1, S2>(
            &self,
            y: &ArrayBase<S1, Ix1>,
            f: &mut ArrayBase<S2, Ix1>,
        ) -> Result<(), failure::Error>
        where
            S1: Data<Elem = <Self as ModelSpec>::Scalar>,
            S2: DataMut<Elem = <Self as ModelSpec>::Scalar>,
        {
            f[0] = y[0].powi(2) + y[1].powi(2) + y[2].powi(2) - 1.0;
            f[1] = 2.0 * y[0].powi(2) + y[1].powi(2) - 4.0 * y[2];
            f[2] = 3.0 * y[0].powi(2) - 4.0 * y[1] + y[2].powi(2);
            Ok(())
        }

        fn lsetup<S1>(
            &mut self,
            y: &ArrayBase<S1, Ix1>,
            _f: &ArrayView<<Self as ModelSpec>::Scalar, Ix1>,
            _jbad: bool,
        ) -> Result<bool, failure::Error>
        where
            S1: Data<Elem = <Self as ModelSpec>::Scalar>,
        {
            // compute the Jacobian
            Self::jac(0.0, y, &Array::zeros(self.model_size()), &mut self.A).map(|_| true)

            // setup the linear solver
            //retval = SUNLinSolSetup(Imem->LS, Imem->A);
        }

        fn lsolve<S1, S2>(
            &self,
            _y: &ArrayBase<S1, Ix1>,
            b: &mut ArrayBase<S2, Ix1>,
        ) -> Result<(), failure::Error>
        where
            S1: Data<Elem = <Self as ModelSpec>::Scalar>,
            S2: DataMut<Elem = <Self as ModelSpec>::Scalar>,
        {
            // Solve self.A * b = b
            //retval = SUNLinSolSolve(Imem->LS, Imem->A, Imem->x, b, ZERO);
            //N_VScale(ONE, Imem->x, b);
            use ndarray_linalg::*;
            self.A
                .solve_inplace(b)
                .map(|_| ())
                .map_err(failure::Error::from)
        }

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
            S3: Data<Elem = <Self as ModelSpec>::Scalar>,
        {
            use crate::traits::NormRms;
            // compute the norm of the correction
            let delnrm = del.norm_wrms(ewt);

            //if (delnrm <= tol) return(SUN_NLS_SUCCESS);  /* success       */
            //else               return(SUN_NLS_CONTINUE); /* not converged */
            Ok(delnrm <= tol)
        }
    }

    #[test]
    fn test_newton() {
        // approximate solution
        let Y = array![
            0.785196933062355226,
            0.496611392944656396,
            0.369922830745872357
        ];

        let mut p = TestProblem {
            A: Array::zeros((3, 3)),
            x: Array::zeros(3),
        };

        // set initial guess
        let y0 = array![0.5, 0.5, 0.5];
        let mut y = Array::zeros(3);

        // set weights
        let w = array![1.0, 1.0, 1.0];

        let mut newton = Newton::new(p.model_size(), 10);
        newton
            .solve(&mut p, &y0, &mut y, &w, 1e-2, true)
            .expect("Should have converged.");

        dbg!(&newton);

        let expected_err = array![-0.00578453, 1.0143e-08, 1.47767e-08];

        // print the solution
        println!("Solution: y = {:?}", y);
        println!("Solution Error = {:?}", &y - &Y);
        println!("Number of nonlinear iterations: {}", newton.niters);

        assert_nearly_eq!(&y - &Y, expected_err, 1e-9);
    }
}