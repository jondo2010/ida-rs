use ndarray::*;
//use ndarray_linalg::*;

use failure::Fail;

use crate::traits::*;

/// hmax_inv default value
const HMAX_INV_DEFAULT: f64 = 0.0;
/// maxord default value
const MAXORD_DEFAULT: usize = 5;
/// max. number of N_Vectors in phi
const MXORDP1: usize = 6;
/// mxstep default value
const MXSTEP_DEFAULT: usize = 500;

/// max number of convergence failures allowed
const MXNCF: u32 = 10;
/// max number of error test failures allowed
const MXNEF: u32 = 10;
/// max. number of h tries in IC calc.
const MAXNH: u32 = 5;
/// max. number of J tries in IC calc.
const MAXNJ: u32 = 4;
/// max. Newton iterations in IC calc.
const MAXNI: u32 = 10;
/// Newton convergence test constant
const EPCON: f64 = 0.33;
/// max backtracks per Newton step in IDACalcIC
const MAXBACKS: u32 = 100;
/// constant for updating Jacobian/preconditioner
const XRATE: f64 = 0.25;

#[derive(Debug, Fail)]
enum IdaError {
    // LSETUP_ERROR_NONRECVR
    // IDA_ERR_FAIL
    /// IDA_REP_RES_ERR:
    #[fail(
        display = "The user's residual function repeatedly returned a recoverable error flag, but the solver was unable to recover"
    )]
    RepeatedResidualError {},

    /// IDA_ILL_INPUT
    #[fail(display = "One of the input arguments was illegal. See printed message")]
    IllegalInput {},

    /// IDA_LINIT_FAIL
    #[fail(display = "The linear solver's init routine failed")]
    LinearInitFail {},

    /// IDA_BAD_EWT
    #[fail(
        display = "Some component of the error weight vector is zero (illegal), either for the input value of y0 or a corrected value"
    )]
    BadErrorWeightVector {},

    /// IDA_RES_FAIL
    #[fail(display = "The user's residual routine returned a non-recoverable error flag")]
    ResidualFail {},

    /// IDA_FIRST_RES_FAIL
    #[fail(
        display = "The user's residual routine returned a recoverable error flag on the first call, but IDACalcIC was unable to recover"
    )]
    FirstResidualFail {},

    /// IDA_LSETUP_FAIL
    #[fail(display = "The linear solver's setup routine had a non-recoverable error")]
    LinearSetupFail {},

    /// IDA_LSOLVE_FAIL
    #[fail(display = "The linear solver's solve routine had a non-recoverable error")]
    LinearSolveFail {},

    /// IDA_NO_RECOVERY
    #[fail(
        display = "The user's residual routine, or the linear solver's setup or solve routine had a recoverable error, but IDACalcIC was unable to recover"
    )]
    NoRecovery {},

    /// IDA_CONSTR_FAIL
    /// The inequality constraints were violated, and the solver was unable to recover.
    #[fail(
        display = "IDACalcIC was unable to find a solution satisfying the inequality constraints"
    )]
    ConstraintFail {},

    /// IDA_LINESEARCH_FAIL
    #[fail(
        display = "The Linesearch algorithm failed to find a  solution with a step larger than steptol   in weighted RMS norm"
    )]
    LinesearchFail {},

    /// IDA_CONV_FAIL
    #[fail(display = "IDACalcIC failed to get convergence of the Newton iterations")]
    ConvergenceFail {},

    ///MSG_BAD_K
    #[fail(display = "Illegal value for k.")]
    BadK {},
    //MSG_NULL_DKY       "dky = NULL illegal."
    ///MSG_BAD_T          
    #[fail(
        display = "Illegal value for t: t = {} is not between tcur - hu = {} and tcur = {}.",
        t, tdiff, tcurr
    )]
    BadTimeValue { t: f64, tdiff: f64, tcurr: f64 },
}

/// Structure containing the parameters for the numerical integration.
#[derive(Debug, Clone)]
pub struct Ida<F: IdaModel> {
    f: F,
    //dt: <F::Scalar as AssociatedReal>::Real,
    //x: Array<F::Scalar, Ix1>,
    /// constraints vector present: do constraints calc
    ida_constraintsSet: bool,
    /// SUNTRUE means suppress algebraic vars in local error tests
    ida_suppressalg: bool,

    // Divided differences array and associated minor arrays
    /// phi = (maxord+1) arrays of divided differences
    ida_phi: Array<F::Scalar, Ix2>,
    /// differences in t (sums of recent step sizes)
    ida_psi: Array1<F::Scalar>,
    /// ratios of current stepsize to psi values
    ida_alpha: Array1<F::Scalar>,
    /// ratios of current to previous product of psi's
    ida_beta: Array1<F::Scalar>,
    /// product successive alpha values and factorial
    ida_sigma: Array1<F::Scalar>,
    /// sum of reciprocals of psi values
    ida_gamma: Array1<F::Scalar>,

    // N_Vectors
    /// error weight vector
    ida_ewt: Array<F::Scalar, Ix1>,
    /// work space for y vector (= user's yret)
    //ida_yy: Array1<<F::Scalar as AssociatedReal>::Real>,
    /// work space for y' vector (= user's ypret)
    //ida_yp: Array1<<F::Scalar as AssociatedReal>::Real>,
    /// predicted y vector
    ida_yypredict: Array<F::Scalar, Ix1>,
    /// predicted y' vector
    ida_yppredict: Array<F::Scalar, Ix1>,
    /// residual vector
    ida_delta: Array<F::Scalar, Ix1>,
    /// bit vector for diff./algebraic components
    ida_id: Array<bool, Ix1>,
    /// vector of inequality constraint options
    //ida_constraints: Array1<<F::Scalar as AssociatedReal>::Real>,
    /// saved residual vector
    //ida_savres: Array1<<F::Scalar as AssociatedReal>::Real>,
    /// accumulated corrections to y vector, but set equal to estimated local errors upon successful return
    ida_ee: Array<F::Scalar, Ix1>,

    //ida_mm;          /* mask vector in constraints tests (= tempv2)    */
    //ida_tempv1;      /* work space vector                              */
    //ida_tempv2;      /* work space vector                              */
    //ida_tempv3;      /* work space vector                              */
    //ida_ynew;        /* work vector for y in IDACalcIC (= tempv2)      */
    //ida_ypnew;       /* work vector for yp in IDACalcIC (= ee)         */
    //ida_delnew;      /* work vector for delta in IDACalcIC (= phi[2])  */
    //ida_dtemp;       /* work vector in IDACalcIC (= phi[3])            */

    // Tstop information
    ida_tstopset: bool,
    ida_tstop: F::Scalar,

    // Step Data
    /// current BDF method order
    ida_kk: usize,
    /// method order used on last successful step
    ida_kused: usize,
    /// order for next step from order decrease decision
    ida_knew: usize,
    /// flag to trigger step doubling in first few steps
    ida_phase: usize,
    /// counts steps at fixed stepsize and order
    ida_ns: usize,

    /// initial step
    ida_hin: F::Scalar,
    /// actual initial stepsize
    ida_h0u: F::Scalar,
    /// current step size h
    //ida_hh: <F::Scalar as AssociatedReal>::Real,
    ida_hh: F::Scalar,
    /// step size used on last successful step
    ida_hused: F::Scalar,
    /// rr = hnext / hused
    ida_rr: F::Scalar,
    //ida_rr: <F::Scalar as AssociatedReal>::Real,
    /// current internal value of t
    ida_tn: F::Scalar,
    /// value of tret previously returned by IDASolve
    ida_tretlast: F::Scalar,
    /// current value of scalar (-alphas/hh) in Jacobian
    ida_cj: F::Scalar,
    /// cj value saved from last successful step
    ida_cjlast: F::Scalar,
    //realtype ida_cjold;    /* cj value saved from last call to lsetup           */
    //realtype ida_cjratio;  /* ratio of cj values: cj/cjold                      */
    //realtype ida_ss;       /* scalar used in Newton iteration convergence test  */
    //realtype ida_oldnrm;   /* norm of previous nonlinear solver update          */
    //realtype ida_epsNewt;  /* test constant in Newton convergence test          */
    //realtype ida_epcon;    /* coeficient of the Newton covergence test          */
    //realtype ida_toldel;   /* tolerance in direct test on Newton corrections    */

    // Limits
    /// max numer of convergence failures
    ida_maxncf: u64,
    /// max number of error test failures
    ida_maxnef: u64,
    /// max value of method order k:
    ida_maxord: usize,
    /// value of maxord used when allocating memory
    //ida_maxord_alloc: u64,
    /// max number of internal steps for one user call
    ida_mxstep: u64,
    /// inverse of max. step size hmax (default = 0.0)
    ida_hmax_inv: F::Scalar,

    // Counters
    /// number of internal steps taken
    ida_nst: u64,
    /// number of function (res) calls
    ida_nre: u64,
    /// number of corrector convergence failures
    ida_ncfn: u64,
    /// number of error test failures
    ida_netf: u64,
    /// number of Newton iterations performed
    ida_nni: u64,
    /// number of lsetup calls
    ida_nsetups: u64,
    // Arrays for Fused Vector Operations
    ida_cvals: Array1<F::Scalar>,
    ida_dvals: Array1<F::Scalar>,

    ida_Xvecs: Array<F::Scalar, Ix2>,
    ida_Zvecs: Array<F::Scalar, Ix2>,
}

impl<
        F: IdaModel<
            Scalar = impl num_traits::Float
                         + num_traits::float::FloatConst
                         + num_traits::NumRef
                         + num_traits::NumAssignRef
                         + ScalarOperand
                         + std::fmt::Debug, /*+ IdaConst*/
        >,
    > Ida<F>
//where
//num_traits::float::Float + num_traits::float::FloatConst + num_traits::NumAssignRef + ScalarOperand
//<<F as ModelSpec>::Dim as Dimension>::Larger: RemoveAxis,
{
    /// Creates a new IdaModel given a ModelSpec, initial Arrays of yy0 and yyp
    ///
    /// *Panics" if ModelSpec::Scalar is unable to convert any constant initialization value.
    pub fn new(f: F, yy0: Array<F::Scalar, Ix1>, yp0: Array<F::Scalar, Ix1>) -> Self {
        // Initialize the phi array
        let mut ida_phi = Array::zeros(f.model_size())
            .broadcast([&[MXORDP1], yy0.shape()].concat())
            .unwrap()
            .into_dimensionality::<_>()
            .unwrap()
            .to_owned();

        ida_phi.index_axis_mut(Axis(0), 0).assign(&yy0);
        ida_phi.index_axis_mut(Axis(0), 1).assign(&yp0);

        //IDAResFn res, realtype t0, N_Vector yy0, N_Vector yp0
        Self {
            f: f,
            // Set unit roundoff in IDA_mem
            // NOTE: Use F::Scalar::epsilon() instead!
            //ida_uround: UNIT_ROUNDOFF,

            // Set default values for integrator optional inputs
            //ida_res:         = NULL,
            //ida_user_data:   = NULL,
            //ida_itol        = IDA_NN;
            //ida_user_efun   = SUNFALSE;
            //ida_efun        = NULL;
            //ida_edata       = NULL;
            //ida_ehfun       = IDAErrHandler;
            //ida_eh_data     = IDA_mem;
            //ida_errfp       = stderr;
            ida_maxord: MAXORD_DEFAULT as usize,
            ida_mxstep: MXSTEP_DEFAULT as u64,
            ida_hmax_inv: F::Scalar::from(HMAX_INV_DEFAULT).unwrap(),
            ida_hin: F::Scalar::zero(),
            //ida_epcon       = EPCON;
            ida_maxnef: MXNEF as u64,
            ida_maxncf: MXNCF as u64,
            //ida_suppressalg = SUNFALSE;
            //ida_id          = NULL;
            //ida_constraints: Array::zeros(yy0.raw_dim()),
            ida_constraintsSet: false,
            ida_tstopset: false,

            // set the saved value maxord_alloc
            //ida_maxord_alloc = MAXORD_DEFAULT;

            // Set default values for IC optional inputs
            //ida_epiccon = PT01 * EPCON;
            //ida_maxnh   = MAXNH;
            //ida_maxnj   = MAXNJ;
            //ida_maxnit  = MAXNI;
            //ida_maxbacks  = MAXBACKS;
            //ida_lsoff   = SUNFALSE;
            //ida_steptol = SUNRpowerR(IDA_mem->ida_uround, TWOTHIRDS);

            /* Initialize lrw and liw */
            //ida_lrw = 25 + 5*MXORDP1;
            //ida_liw = 38;

            /* Initialize nonlinear solver pointer */
            //IDA_mem->NLS    = NULL;
            //IDA_mem->ownNLS = SUNFALSE;
            ida_phi: ida_phi,

            ida_psi: Array::zeros(MXORDP1),
            ida_alpha: Array::zeros(MXORDP1),
            ida_beta: Array::zeros(MXORDP1),
            ida_sigma: Array::zeros(MXORDP1),
            ida_gamma: Array::zeros(MXORDP1),

            ida_delta: Array::zeros(yy0.raw_dim()),
            ida_id: Array::from_elem(yy0.raw_dim(), false),

            // Initialize all the counters and other optional output values
            ida_nst: 0,
            ida_nre: 0,
            ida_ncfn: 0,
            ida_netf: 0,
            ida_nni: 0,
            ida_nsetups: 0,
            ida_kused: 0,
            ida_hused: F::Scalar::zero(),
            //ida_tolsf: <F::Scalar as AssociatedReal>::Real::from_f64(1.0),

            //ida_nge = 0;

            //ida_irfnd = 0;

            // Initialize root-finding variables

            //ida_glo     = NULL;
            //ida_ghi     = NULL;
            //ida_grout   = NULL;
            //ida_iroots  = NULL;
            //ida_rootdir = NULL;
            //ida_gfun    = NULL;
            //ida_nrtfn   = 0;
            //ida_gactive  = NULL;
            //ida_mxgnull  = 1;

            // Not from ida.c...
            ida_ewt: Array::zeros(yy0.raw_dim()),
            ida_ee: Array::zeros(yy0.raw_dim()),
            ida_suppressalg: false,

            ida_tstop: F::Scalar::zero(),

            ida_kk: 0,
            //ida_kused: 0,
            ida_knew: 0,
            ida_phase: 0,
            ida_ns: 0,

            ida_rr: F::Scalar::zero(),
            ida_tn: F::Scalar::zero(),
            ida_tretlast: F::Scalar::zero(),
            ida_h0u: F::Scalar::zero(),
            ida_hh: F::Scalar::zero(),
            //ida_hused: <F::Scalar as AssociatedReal>::Real::from_f64(0.0),
            ida_cj: F::Scalar::zero(),
            ida_cjlast: F::Scalar::zero(),

            ida_cvals: Array::zeros(MXORDP1),
            ida_dvals: Array::zeros(MAXORD_DEFAULT),

            ida_Xvecs: Array::zeros((MXORDP1, yy0.shape()[0])),
            ida_Zvecs: Array::zeros((MXORDP1, yy0.shape()[0])),

            ida_yypredict: Array::zeros(yy0.raw_dim()),
            ida_yppredict: Array::zeros(yy0.raw_dim()),
        }
    }

    /// This routine performs one internal IDA step, from tn to tn + hh. It calls other routines to do all the work.
    ///
    /// It solves a system of differential/algebraic equations of the form F(t,y,y') = 0, for one step.
    /// In IDA, tt is used for t, yy is used for y, and yp is used for y'. The function F is supplied
    /// as 'res' by the user.
    ///
    /// The methods used are modified divided difference, fixed leading coefficient forms of backward
    /// differentiation formulas. The code adjusts the stepsize and order to control the local error
    /// per step.
    ///
    /// The main operations done here are as follows:
    /// * initialize various quantities;
    /// * setting of multistep method coefficients;
    /// * solution of the nonlinear system for yy at t = tn + hh;
    /// * deciding on order reduction and testing the local error;
    /// * attempting to recover from failure in nonlinear solver or error test;
    /// * resetting stepsize and order for the next step.
    /// * updating phi and other state data if successful;
    ///
    /// On a failure in the nonlinear system solution or error test, the step may be reattempted,
    /// depending on the nature of the failure.
    ///
    /// Variables or arrays (all in the IDAMem structure) used in IDAStep are:
    ///
    /// tt -- Independent variable.
    /// yy -- Solution vector at tt.
    /// yp -- Derivative of solution vector after successful stelp.
    /// res -- User-supplied function to evaluate the residual. See the description given in file ida.h
    /// lsetup -- Routine to prepare for the linear solver call. It may either save or recalculate
    ///   quantities used by lsolve. (Optional)
    /// lsolve -- Routine to solve a linear system. A prior call to lsetup may be required.
    /// hh  -- Appropriate step size for next step.
    /// ewt -- Vector of weights used in all convergence tests.
    /// phi -- Array of divided differences used by IDAStep. This array is composed of (maxord+1)
    ///   nvectors (each of size Neq). (maxord+1) is the maximum order for the problem, maxord, plus 1.
    ///
    /// Return values are:
    ///       IDA_SUCCESS   IDA_RES_FAIL      LSETUP_ERROR_NONRECVR
    ///                     IDA_LSOLVE_FAIL   IDA_ERR_FAIL
    ///                     IDA_CONSTR_FAIL   IDA_CONV_FAIL
    ///                     IDA_REP_RES_ERR
    fn step(&mut self) -> Result<(), failure::Error> {
        //realtype saved_t, ck;
        //realtype err_k, err_km1;
        //int ncf, nef;
        //int nflag, kflag;
        let mut ck = F::Scalar::one();

        let saved_t = self.ida_tn;
        //ncf = nef = 0;

        if self.ida_nst == 0 {
            self.ida_kk = 1;
            self.ida_kused = 0;
            self.ida_hused = F::Scalar::one();
            self.ida_psi[0] = self.ida_hh;
            self.ida_cj = F::Scalar::one() / self.ida_hh;
            self.ida_phase = 0;
            self.ida_ns = 0;
        }

        /* To prevent 'unintialized variable' warnings */
        //err_k = ZERO;
        //err_km1 = ZERO;

        /* Looping point for attempts to take a step */

        loop {
            //-----------------------
            // Set method coefficients
            //-----------------------

            ck = self.set_coeffs();

            //kflag = IDA_SUCCESS;

            //----------------------------------------------------
            // If tn is past tstop (by roundoff), reset it to tstop.
            //-----------------------------------------------------

            self.ida_tn += self.ida_hh;
            if self.ida_tstopset {
                if (self.ida_tn - self.ida_tstop) * self.ida_hh > F::Scalar::one() {
                    self.ida_tn = self.ida_tstop;
                }
            }

            //-----------------------
            // Advance state variables
            //-----------------------

            // Compute predicted values for yy and yp
            self.predict();

            // Nonlinear system solution
            let nflag = self.nonlinear_solve();

            // If NLS was successful, perform error test
            if nflag.is_ok() {
                let (err_k, err_km1, nflag) = self.test_error(ck);
            }

            // Test for convergence or error test failures
            //if nflag != IDA_SUCCESS {
            // restore and decide what to do
            self.restore(saved_t);
            //kflag = handle_n_flag(IDA_mem, nflag, err_k, err_km1, &(self.ida_ncfn), &ncf, &(self.ida_netf), &nef);

            // exit on nonrecoverable failure
            //if kflag != PREDICT_AGAIN {
            //    return (kflag);
            //}

            // recoverable error; predict again
            if self.ida_nst == 0 {
                self.reset();
            }
            continue;
            //}

            /* kflag == IDA_SUCCESS */
            break;
        }

        /* Nonlinear system solve and error test were both successful;
        update data, and consider change of step and/or order */

        //self.complete_step(err_k, err_km1);

        /*
          Rescale ee vector to be the estimated local error
          Notes:
            (1) altering the value of ee is permissible since
                it will be overwritten by
                IDASolve()->IDAStep()->IDANls()
                before it is needed again
            (2) the value of ee is only valid if IDAHandleNFlag()
                returns either PREDICT_AGAIN or IDA_SUCCESS
        */
        //N_VScale(ck, IDA_mem->ida_ee, IDA_mem->ida_ee);
        self.ida_ee *= ck;

        Ok(())
    }

    /// This routine computes the coefficients relevant to the current step.
    ///
    /// The counter ns counts the number of consecutive steps taken at constant stepsize h and order
    /// k, up to a maximum of k + 2.
    /// Then the first ns components of beta will be one, and on a step with ns = k + 2, the
    /// coefficients alpha, etc. need not be reset here.
    /// Also, complete_step() prohibits an order increase until ns = k + 2.
    ///
    /// Returns the 'variable stepsize error coefficient ck'
    pub fn set_coeffs(&mut self) -> F::Scalar {
        // Set coefficients for the current stepsize h
        if self.ida_hh != self.ida_hused || self.ida_kk != self.ida_kused {
            self.ida_ns = 0;
        }
        self.ida_ns = std::cmp::min(self.ida_ns + 1, self.ida_kused + 2);
        if self.ida_kk + 1 >= self.ida_ns {
            self.ida_beta[0] = F::Scalar::one();
            self.ida_alpha[0] = F::Scalar::one();
            let mut temp1 = self.ida_hh;
            self.ida_gamma[0] = F::Scalar::zero();
            self.ida_sigma[0] = F::Scalar::one();
            for i in 1..self.ida_kk {
                let temp2 = self.ida_psi[i - 1];
                self.ida_psi[i - 1] = temp1;
                self.ida_beta[i] = self.ida_beta[i - 1] * (self.ida_psi[i - 1] / temp2);
                temp1 = temp2 + self.ida_hh;
                self.ida_alpha[i] = self.ida_hh / temp1;
                self.ida_sigma[i] =
                    self.ida_sigma[i - 1] * self.ida_alpha[i] * F::Scalar::from(i).unwrap();
                self.ida_gamma[i] = self.ida_gamma[i - 1] + self.ida_alpha[i - 1] / self.ida_hh;
            }
            self.ida_psi[self.ida_kk] = temp1;
        }
        // compute alphas, alpha0
        let mut alphas = F::Scalar::zero();
        let mut alpha0 = F::Scalar::zero();
        for i in 0..self.ida_kk {
            alphas -= F::Scalar::one() / F::Scalar::from(i + 1).unwrap();
            alpha0 -= self.ida_alpha[i];
        }

        // compute leading coefficient cj
        self.ida_cjlast = self.ida_cj;
        self.ida_cj = -alphas / self.ida_hh;

        // compute variable stepsize error coefficient ck
        let mut ck = (self.ida_alpha[self.ida_kk] + alphas - alpha0).abs();
        ck = ck.max(self.ida_alpha[self.ida_kk]);

        // change phi to phi-star
        // Scale i=self.ida_ns to i<=self.ida_kk
        if self.ida_ns <= self.ida_kk {
            let nv = self.ida_kk - self.ida_ns + 1;
            let c = self.ida_beta.slice(s![self.ida_ns..]);

            let ix1 = s![self.ida_ns..];
            let ix2 = SliceOrIndex::from(0..self.ida_ns);
            //self.ida_phi.slice(ix1);
            //&SliceInfo<<<<F as ModelSpec>::Dim as Dimension>::Larger as Dimension>::SliceArg, _>
            //let ix: <<<F as ModelSpec>::Dim as Dimension>::Larger as Dimension>::SliceArg = &[SliceOrIndex::from(0..self.ida_ns)];

            //<<<<F as ModelSpec>::Dim as Dimension>::Larger as Dimension>::SliceArg as AsRef<[SliceOrIndex]>>::from(0..1);

            //let z = self.ida_phi.slice_mut(s![self.ida_ns..]);
            //self.ida_phi.index_axis_mut(Axis(0), 0).assign(&yy0);
            /*
            N_VScaleVectorArray(
              self.ida_kk - self.ida_ns + 1,
              self.ida_beta + self.ida_ns,
              self.ida_phi + self.ida_ns,
              self.ida_phi + self.ida_ns,
            );
            */
        }

        return ck;
    }

    /// IDANls
    /// This routine attempts to solve the nonlinear system using the linear solver specified.
    /// NOTE: this routine uses N_Vector ee as the scratch vector tempv3 passed to lsetup.
    pub fn nonlinear_solve(&mut self) -> Result<(), failure::Error> {
        unimplemented!();
    }

    /// IDAPredict
    /// This routine predicts the new values for vectors yy and yp.
    pub fn predict(&mut self) -> () {
        for j in 0..self.ida_kk {
            self.ida_cvals[j] = F::Scalar::one();
        }

        let cv = self.ida_cvals.slice(s![self.ida_kk + 1..]);
        let ph = self.ida_phi.index_axis(Axis(0), self.ida_kk + 1);
        //let x = self.ida_phi.slice(s![self.ida_kk + 1.., ..]);

        // ida_delta = ida_phi[ida_kk] + self.ida_ee;
        //self.ida_delta .assign(&self.ida_phi.index_axis(Axis(0), self.ida_kk));
        let v = self.ida_phi.index_axis(Axis(0), self.ida_kk);
        //Zip::from(&mut self.ida_delta).and(&v).and(&self.ida_ee).apply(|delta, phi, ee| {});
        self.ida_delta.assign(&v);
        self.ida_delta += &self.ida_ee;

        //self.ida_delta = &v + &self.ida_ee;

        // ida_yypredict = sum 0..kk (cvals[k] * phi[k])
        /*
        for i = 0..n
            for j = 0..nv

        */

        let c = self.ida_cvals.slice(s![self.ida_kk + 1..]);
        let x = self
            .ida_phi
            .slice_axis(Axis(0), Slice::from(self.ida_kk + 1..));
        //let mut z = self.ida_yypredict.slice_axis_mut(Axis(0), Slice::from(self.ida_kk+1..));

        ndarray::Zip::from(&mut self.ida_yypredict)
            .and(x.lanes(Axis(0)))
            .apply(|z, row| {
                *z = (&row * &c).sum();
            });

        //N_VLinearCombination(&c, &x, &mut z);
        //(void) N_VLinearCombination(IDA_mem->ida_kk+1, IDA_mem->ida_cvals, IDA_mem->ida_phi, IDA_mem->ida_yypredict);
        //(void) N_VLinearCombination(IDA_mem->ida_kk, IDA_mem->ida_gamma+1, IDA_mem->ida_phi+1, IDA_mem->ida_yppredict);
    }

    /// IDATestError
    ///
    /// This routine estimates errors at orders k, k-1, k-2, decides whether or not to suggest an order
    /// decrease, and performs the local error test.
    ///
    /// Returns a tuple of (err_k, err_km1, nflag)
    pub fn test_error(
        &mut self,
        ck: F::Scalar,
    ) -> (
        F::Scalar, // err_k
        F::Scalar, // err_km1
        bool,      // nflag
    ) {
        //realtype enorm_k, enorm_km1, enorm_km2;   /* error norms */
        //realtype terr_k, terr_km1, terr_km2;      /* local truncation error norms */
        // Compute error for order k.
        let enorm_k = self.wrms_norm(&self.ida_ee, &self.ida_ewt, self.ida_suppressalg);
        let err_k = self.ida_sigma[self.ida_kk] * enorm_k;
        let terr_k = err_k * F::Scalar::from(self.ida_kk + 1).unwrap();

        let mut err_km1 = F::Scalar::zero(); // estimated error at k-1
        let mut err_km2 = F::Scalar::zero(); // estimated error at k-2

        self.ida_knew = self.ida_kk;

        if self.ida_kk > 1 {
            // Compute error at order k-1
            self.ida_delta = &self.ida_phi.index_axis(Axis(0), self.ida_kk) + &self.ida_ee;
            let enorm_km1 = self.wrms_norm(&self.ida_delta, &self.ida_ewt, self.ida_suppressalg);
            err_km1 = self.ida_sigma[self.ida_kk - 1] * enorm_km1;
            let terr_km1 = err_km1 * F::Scalar::from(self.ida_kk).unwrap();

            if self.ida_kk > 2 {
                // Compute error at order k-2
                // ida_delta = ida_phi[ida_kk - 1] + ida_delta
                self.ida_delta
                    .assign(&self.ida_phi.index_axis(Axis(0), self.ida_kk - 1));
                self.ida_delta.scaled_add(F::Scalar::one(), &self.ida_ee);

                let enorm_km2 =
                    self.wrms_norm(&self.ida_delta, &self.ida_ewt, self.ida_suppressalg);
                err_km2 = self.ida_sigma[self.ida_kk - 2] * enorm_km2;
                let terr_km2 = err_km2 * F::Scalar::from(self.ida_kk - 1).unwrap();

                // Decrease order if errors are reduced
                if terr_km1.max(terr_km2) <= terr_k {
                    self.ida_knew = self.ida_kk - 1;
                }
            } else {
                // Decrease order to 1 if errors are reduced by at least 1/2
                if terr_km1 <= (terr_k * F::Scalar::from(0.5).unwrap()) {
                    self.ida_knew = self.ida_kk - 1;
                }
            }
        };

        (
            err_k,
            err_km1,
            (ck * enorm_k) > F::Scalar::one(), // Perform error test
        )
    }

    /// IDARestore
    /// This routine restores tn, psi, and phi in the event of a failure.
    /// It changes back `phi-star` to `phi` (changed in `set_coeffs()`)
    ///
    ///
    pub fn restore(&mut self, saved_t: F::Scalar) -> () {
        self.ida_tn = saved_t;

        // Restore psi[0 .. kk] = psi[1 .. kk + 1] - hh
        for j in 1..self.ida_kk + 1 {
            self.ida_psi[j - 1] = self.ida_psi[j] - self.ida_hh;
        }

        //Zip::from(&mut self.ida_psi.slice_mut(s![0..self.ida_kk]))
        //.and(&self.ida_psi.slice(s![1..self.ida_kk+1]));
        //ida_psi -= &self.ida_psi.slice(s![1..self.ida_kk+1]);

        if self.ida_ns <= self.ida_kk {
            // cvals[0 .. kk-ns+1] = 1 / beta[ns .. kk+1]
            Zip::from(
                &mut self
                    .ida_cvals
                    .slice_mut(s![0..self.ida_kk - self.ida_ns + 1]),
            )
            .and(&self.ida_beta.slice(s![self.ida_ns..self.ida_kk + 1]))
            .apply(|cvals, &beta| {
                *cvals = beta.recip();
            });

            // phi[ns .. (kk + 1)] *= cvals[ns .. (kk + 1)]
            let mut ida_phi = self
                .ida_phi
                .slice_axis_mut(Axis(0), Slice::from(self.ida_ns..self.ida_kk + 1));

            // We manually broadcast cvals here so we can turn it into a column vec
            let cvals = self.ida_cvals.slice(s![0..self.ida_kk - self.ida_ns + 1]);
            let cvals = cvals
                .broadcast((1, ida_phi.len_of(Axis(0))))
                .unwrap()
                .reversed_axes();

            ida_phi *= &cvals;
        }
    }

    /// IDAHandleNFlag
    /// This routine handles failures indicated by the input variable nflag. Positive values indicate various recoverable failures while negative values indicate nonrecoverable failures. This routine adjusts the step size for recoverable failures.
    ///
    ///  Possible nflag values (input):
    ///
    ///   --convergence failures--
    ///   IDA_RES_RECVR              > 0
    ///   IDA_LSOLVE_RECVR           > 0
    ///   IDA_CONSTR_RECVR           > 0
    ///   SUN_NLS_CONV_RECV          > 0
    ///   IDA_RES_FAIL               < 0
    ///   IDA_LSOLVE_FAIL            < 0
    ///   IDA_LSETUP_FAIL            < 0
    ///
    ///   --error test failure--
    ///   ERROR_TEST_FAIL            > 0
    ///
    ///  Possible kflag values (output):
    ///
    ///   --recoverable--
    ///   PREDICT_AGAIN
    ///
    ///   --nonrecoverable--
    ///   IDA_CONSTR_FAIL
    ///   IDA_REP_RES_ERR
    ///   IDA_ERR_FAIL
    ///   IDA_CONV_FAIL
    ///   IDA_RES_FAIL
    ///   IDA_LSETUP_FAIL
    ///   IDA_LSOLVE_FAIL
    pub fn handle_n_flag(
        &mut self,
        nflag: u32,
        err_k: F::Scalar,
        err_km1: F::Scalar, //long int *ncfnPtr,
                            //int *ncfPtr,
                            //long int *netfPtr,
                            //int *nefPtr
    ) -> () {
        unimplemented!();
    }

    /// IDAReset
    /// This routine is called only if we need to predict again at the very first step. In such a case,
    /// reset phi[1] and psi[0].
    pub fn reset(&mut self) -> () {
        self.ida_psi[0] = self.ida_hh;
        //N_VScale(IDA_mem->ida_rr, IDA_mem->ida_phi[1], IDA_mem->ida_phi[1]);
        self.ida_phi *= self.ida_rr;
    }

    /// IDACompleteStep
    /// This routine completes a successful step.  It increments nst, saves the stepsize and order
    /// used, makes the final selection of stepsize and order for the next step, and updates the phi
    /// array.
    pub fn complete_step(&mut self, err_k: F::Scalar, err_km1: F::Scalar) -> () {
        self.ida_nst += 1;
        let kdiff = self.ida_kk - self.ida_kused;
        self.ida_kused = self.ida_kk;
        self.ida_hused = self.ida_hh;

        if (self.ida_knew == self.ida_kk - 1) || (self.ida_kk == self.ida_maxord) {
            self.ida_phase = 1;
        }

        // For the first few steps, until either a step fails, or the order is reduced, or the
        // order reaches its maximum, we raise the order and double the stepsize. During these
        // steps, phase = 0. Thereafter, phase = 1, and stepsize and order are set by the usual
        // local error algorithm.
        //
        // Note that, after the first step, the order is not increased, as not all of the
        // neccessary information is available yet.

        if self.ida_phase == 0 {
            if self.ida_nst > 1 {
                self.ida_kk += 1;
                let mut hnew = F::Scalar::from(2.0).unwrap() * self.ida_hh;
                let tmp = hnew.abs() * self.ida_hmax_inv;
                if tmp > F::Scalar::one() {
                    hnew /= tmp;
                }
                self.ida_hh = hnew;
            }
        } else {
            enum Action {
                None,
                Lower,
                Maintain,
                Raise,
            }

            let mut action = Action::None;

            // Set action = LOWER/MAINTAIN/RAISE to specify order decision

            if self.ida_knew == (self.ida_kk - 1) {
                action = Action::Lower;
            } else if self.ida_kk == self.ida_maxord {
                action = Action::Maintain;
            } else if (self.ida_kk + 1) >= self.ida_ns || (kdiff == 1) {
                action = Action::Maintain;
            }

            // Estimate the error at order k+1, unless already decided to reduce order, or already using
            // maximum order, or stepsize has not been constant, or order was just raised.

            let mut err_kp1 = F::Scalar::zero();

            if let Action::None = action {
                //N_VLinearSum(ONE, IDA_mem->ida_ee, -ONE, IDA_mem->ida_phi[IDA_mem->ida_kk + 1], IDA_mem->ida_tempv1);
                let ida_tempv1 = &self.ida_ee - &self.ida_phi.index_axis(Axis(0), self.ida_kk + 1);
                let enorm = self.wrms_norm(&ida_tempv1, &self.ida_ewt, self.ida_suppressalg);
                err_kp1 = enorm / F::Scalar::from(self.ida_kk + 2).unwrap();

                // Choose among orders k-1, k, k+1 using local truncation error norms.

                let terr_k = F::Scalar::from(self.ida_kk + 1).unwrap() * err_k;
                let terr_kp1 = F::Scalar::from(self.ida_kk + 2).unwrap() * err_kp1;

                if self.ida_kk == 1 {
                    if terr_kp1 >= F::Scalar::from(0.5).unwrap() * terr_k {
                        action = Action::Maintain;
                    } else {
                        action = Action::Raise;
                    }
                } else {
                    let terr_km1 = F::Scalar::from(self.ida_kk).unwrap() * err_km1;
                    if terr_km1 <= terr_k.min(terr_kp1) {
                        action = Action::Lower;
                    } else if terr_kp1 >= terr_k {
                        action = Action::Maintain;
                    } else {
                        action = Action::Raise;
                    }
                }
            }
            //takeaction:

            // Set the estimated error norm and, on change of order, reset kk.
            let err_knew = match action {
                Action::Raise => {
                    self.ida_kk += 1;
                    err_kp1
                }
                Action::Lower => {
                    self.ida_kk -= 1;
                    err_km1
                }
                _ => err_k,
            };

            // Compute rr = tentative ratio hnew/hh from error norm estimate.
            // Reduce hh if rr <= 1, double hh if rr >= 2, else leave hh as is.
            // If hh is reduced, hnew/hh is restricted to be between .5 and .9.

            let mut hnew = self.ida_hh;
            //ida_rr = SUNRpowerR( TWO * err_knew + PT0001, -ONE/(IDA_mem->ida_kk + 1) );
            self.ida_rr = {
                let base =
                    F::Scalar::from(2.0).unwrap() * err_knew + F::Scalar::from(0.0001).unwrap();
                let arg =
                    -F::Scalar::one() / (F::Scalar::from(self.ida_kk).unwrap() + F::Scalar::one());
                base.powf(arg)
            };

            if self.ida_rr >= F::Scalar::from(2.0).unwrap() {
                hnew = F::Scalar::from(2.0).unwrap() * self.ida_hh;
                let tmp = hnew.abs() * self.ida_hmax_inv;
                if tmp > F::Scalar::one() {
                    hnew /= tmp;
                }
            } else if self.ida_rr <= F::Scalar::one() {
                //ida_rr = SUNMAX(HALF, SUNMIN(PT9,IDA_mem->ida_rr));
                self.ida_rr = F::Scalar::from(0.5)
                    .unwrap()
                    .max(self.ida_rr.min(F::Scalar::from(0.9).unwrap()));
                hnew = self.ida_hh * self.ida_rr;
            }

            self.ida_hh = hnew;
        }
        // end of phase if block

        // Save ee for possible order increase on next step
        if self.ida_kused < self.ida_maxord {
            //N_VScale(ONE, IDA_mem->ida_ee, IDA_mem->ida_phi[IDA_mem->ida_kused + 1]);
            self.ida_phi
                .index_axis_mut(Axis(0), self.ida_kused + 1)
                .assign(&self.ida_ee);
        }

        // Update phi arrays

        // To update phi arrays compute X += Z where                  */
        // X = [ phi[kused], phi[kused-1], phi[kused-2], ... phi[1] ] */
        // Z = [ ee,         phi[kused],   phi[kused-1], ... phi[0] ] */
        self.ida_Zvecs
            .index_axis_mut(Axis(0), 0)
            .assign(&self.ida_ee);
        self.ida_Zvecs
            .slice_mut(s![1..self.ida_kused + 1, ..])
            .assign(&self.ida_phi.slice(s![1..self.ida_kused + 1;-1, ..]));
        self.ida_Xvecs
            .slice_mut(s![1..self.ida_kused + 1, ..])
            .assign(&self.ida_phi.slice(s![0..self.ida_kused;-1, ..]));

        let mut sliceXvecs = self
            .ida_Xvecs
            .slice_axis_mut(Axis(0), Slice::from(0..self.ida_kused + 1));
        let sliceZvecs = self
            .ida_Zvecs
            .slice_axis(Axis(0), Slice::from(0..self.ida_kused + 1));
        sliceXvecs += &sliceZvecs;
    }

    /// This routine evaluates `y(t)` and `y'(t)` as the value and derivative of the interpolating
    /// polynomial at the independent variable t, and stores the results in the vectors yret and ypret.
    /// It uses the current independent variable value, tn, and the method order last used, kused.
    /// This function is called by `solve` with `t = tout`, `t = tn`, or `t = tstop`.
    ///
    /// If `kused = 0` (no step has been taken), or if `t = tn`, then the order used here is taken
    /// to be 1, giving `yret = phi[0]`, `ypret = phi[1]/psi[0]`.
    ///
    /// The return values are:
    ///   IDA_SUCCESS  if t is legal, or
    ///   IDA_BAD_T    if t is not within the interval of the last step taken.
    pub fn get_solution(
        &mut self,
        t: F::Scalar,
        yret: &mut Array<F::Scalar, Ix1>,
        ypret: &mut Array<F::Scalar, Ix1>,
    ) -> Result<(), failure::Error> {
        // Check t for legality.  Here tn - hused is t_{n-1}.

        //tfuzz = HUNDRED * IDA_mem->ida_uround * (SUNRabs(IDA_mem->ida_tn) + SUNRabs(IDA_mem->ida_hh));

        let mut tfuzz = F::Scalar::from(100.0).unwrap()
            * F::Scalar::epsilon()
            * (self.ida_tn.abs() + self.ida_hh.abs());
        if self.ida_hh < F::Scalar::zero() {
            tfuzz = -tfuzz;
        }
        let tp = self.ida_tn - self.ida_hused - tfuzz;
        if (t - tp) * self.ida_hh < F::Scalar::zero() {
            Err(IdaError::BadTimeValue {
                t: t.to_f64().unwrap(),
                tdiff: (self.ida_tn - self.ida_hused).to_f64().unwrap(),
                tcurr: self.ida_tn.to_f64().unwrap(),
            })?;
        }

        // Initialize kord = (kused or 1).
        let kord = if self.ida_kused == 0 {
            1
        } else {
            self.ida_kused
        };

        // Accumulate multiples of columns phi[j] into yret and ypret.
        let delt = t - self.ida_tn;
        let mut c = F::Scalar::one();
        let mut d = F::Scalar::zero();
        let mut gam = delt / self.ida_psi[0];

        self.ida_cvals[0] = c;
        for j in 1..kord {
            d = d * gam + c / self.ida_psi[j - 1];
            c = c * gam;
            gam = (delt + self.ida_psi[j - 1]) / self.ida_psi[j];

            self.ida_cvals[j] = c;
            self.ida_dvals[j - 1] = d;
        }

        //retval = N_VLinearCombination(kord+1, IDA_mem->ida_cvals, IDA_mem->ida_phi,  yret);
        ndarray::Zip::from(yret)
            .and(
                self.ida_phi
                    .slice_axis(Axis(0), Slice::from(0..kord + 1))
                    .lanes(Axis(0)),
            )
            .apply(|z, row| {
                *z = (&row * &self.ida_cvals.slice(s![0..kord + 1])).sum();
            });

        //retval = N_VLinearCombination(kord, IDA_mem->ida_dvals, IDA_mem->ida_phi+1, ypret);
        ndarray::Zip::from(ypret)
            .and(
                self.ida_phi
                    .slice_axis(Axis(0), Slice::from(1..kord + 1))
                    .lanes(Axis(0)),
            )
            .apply(|z, row| {
                *z = (&row * &self.ida_dvals.slice(s![0..kord])).sum();
            });

        Ok(())
    }

    /// Returns the WRMS norm of vector x with weights w.
    /// If mask = SUNTRUE, the weight vector w is masked by id, i.e.,
    ///      nrm = N_VWrmsNormMask(x,w,id);
    ///  Otherwise,
    ///      nrm = N_VWrmsNorm(x,w);
    ///
    /// mask = SUNFALSE       when the call is made from the nonlinear solver.
    /// mask = suppressalg otherwise.
    pub fn wrms_norm(
        &self,
        x: &Array<F::Scalar, Ix1>,
        w: &Array<F::Scalar, Ix1>,
        mask: bool,
    ) -> F::Scalar {
        if mask {
            //x.norm_wrms_masked(w, self.ida_id)
            x.norm_wrms_masked(w, &self.ida_id)
        } else {
            x.norm_wrms(w)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ida::Ida;
    use crate::lorenz63::Lorenz63;
    use ndarray::*;
    use nearly_eq::*;

    #[test]
    fn test_test_error1() {
        let ck = 1.091414141414142;
        let suppressalg = 0;
        let kk = 5;
        let ida_phi = array![
            [
                3.634565317158998e-05,
                1.453878335134203e-10,
                0.9999636542014404,
            ],
            [
                -6.530333550677049e-06,
                -2.612329458968465e-11,
                6.530359673556191e-06,
            ],
            [
                1.946442728026142e-06,
                7.786687275994346e-12,
                -1.946450515496441e-06,
            ],
            [
                -8.097632208221231e-07,
                -3.239585549038764e-12,
                8.097664556005615e-07,
            ],
            [
                3.718130977075839e-07,
                1.487573462300438e-12,
                -3.71814615793545e-07,
            ],
            [
                -3.24421895454213e-07,
                -1.297915245220823e-12,
                3.244230624265827e-07,
            ],
        ];
        let ida_ee = array![
            2.65787533317467e-07,
            1.063275845801634e-12,
            -2.657884288386138e-07,
        ];
        let ida_ewt = array![73343005.56993243, 999999.985461217, 9901.346408259429];
        let ida_sigma = array![
            1.0,
            0.6666666666666666,
            0.6666666666666666,
            0.888888888888889,
            1.422222222222222,
            2.585858585858586,
        ];
        let knew = 4;
        let err_k = 29.10297975314245;
        let err_km1 = 3.531162835377502;
        let nflag = true;

        let f = Lorenz63::default();
        let mut ida = Ida::new(f, array![0., 0., 0.], array![0., 0., 0.]);

        // Set preconditions:
        ida.ida_kk = kk;
        ida.ida_suppressalg = suppressalg > 0;
        ida.ida_phi.assign(&ida_phi);
        ida.ida_ee.assign(&ida_ee);
        ida.ida_ewt.assign(&ida_ewt);
        ida.ida_sigma.assign(&ida_sigma);

        // Call the function under test
        let (err_k_new, err_km1_new, nflag_new) = ida.test_error(ck);

        assert_eq!(ida.ida_knew, knew);
        assert_nearly_eq!(err_k_new, err_k);
        assert_nearly_eq!(err_km1_new, err_km1);
        assert_eq!(nflag_new, nflag);
    }

    #[test]
    fn test_test_error2() {
        //--- IDATestError Before:
        let ck = 0.2025812352167927;
        let suppressalg = 0;
        let kk = 4;
        let ida_phi = array![
            [
                3.051237735052657e-05,
                1.220531905117091e-10,
                0.9999694875005963,
            ],
            [
                -2.513114849098281e-06,
                -1.005308974226734e-11,
                2.513124902721765e-06,
            ],
            [
                4.500284453718991e-07,
                1.800291970640913e-12,
                -4.500302448499092e-07,
            ],
            [
                -1.366709389821433e-07,
                -5.467603693902342e-13,
                1.366714866794709e-07,
            ],
            [
                7.278821769100639e-08,
                2.911981566628798e-13,
                -7.278850816613011e-08,
            ],
            [
                -8.304741244343501e-09,
                -3.324587131187576e-14,
                8.304772990651073e-09,
            ],
        ];
        let ida_ee = array![
            -2.981302228744271e-08,
            -1.192712676406388e-13,
            2.981313872620108e-08,
        ];
        let ida_ewt = array![76621085.31777237, 999999.9877946811, 9901.289220872719,];
        let ida_sigma = array![
            1.0,
            0.5,
            0.3214285714285715,
            0.2396514200444849,
            0.1941955227762807,
            2.585858585858586,
        ];
        //--- IDATestError After:
        let knew = 4;
        let err_k = 0.2561137489433976;
        let err_km1 = 0.455601916633899;
        let nflag = false;

        let f = Lorenz63::default();
        let mut ida = Ida::new(f, array![0., 0., 0.], array![0., 0., 0.]);

        // Set preconditions:
        ida.ida_kk = kk;
        ida.ida_suppressalg = suppressalg > 0;
        ida.ida_phi.assign(&ida_phi);
        ida.ida_ee.assign(&ida_ee);
        ida.ida_ewt.assign(&ida_ewt);
        ida.ida_sigma.assign(&ida_sigma);

        // Call the function under test
        let (err_k_new, err_km1_new, nflag_new) = ida.test_error(ck);

        assert_eq!(ida.ida_knew, knew);
        assert_nearly_eq!(err_k_new, err_k);
        assert_nearly_eq!(err_km1_new, err_km1);
        assert_eq!(nflag_new, nflag);
    }

    #[test]
    fn test_restore1() {
        let saved_t = 717553.4942644858;
        #[rustfmt::skip]
        let phi_before = array![[0.00280975951420059, 1.125972706132338e-08, 0.9971902292261264], [-0.0001926545663078034, -7.857235149861102e-10,0.0001926553520857565], [2.945636347837807e-05, 1.066748079583829e-10,-2.945647009050819e-05], [-5.518529121250618e-06, -4.529997656241677e-11,5.518574540464112e-06], [2.822681468681011e-06, -4.507342025411469e-11,-2.822636100488049e-06], [-8.124641701620927e-08,-8.669560754165103e-11,8.133355922669991e-08], ];
        #[rustfmt::skip]
        let psi_before = array![ 47467.05706123715, 94934.1141224743, 142401.1711837114, 166134.69971433, 189868.2282449486, 107947.0192373629 ];
        let cvals_before = array![1., 1., 1., 1., 1., 0.];
        let beta_before = array![1., 1., 1., 1.2, 1.4, 1.];

        #[rustfmt::skip]
        let phi_after = array![[0.00280975951420059,1.125972706132338e-08, 0.9971902292261264,], [-0.0001926545663078034,-7.857235149861102e-10,0.0001926553520857565,], [2.945636347837807e-05,1.066748079583829e-10,-2.945647009050819e-05,], [-4.598774267708849e-06,-3.774998046868064e-11,4.598812117053426e-06,], [2.016201049057865e-06,-3.219530018151049e-11,-2.016168643205749e-06,], [-8.124641701620927e-08,-8.669560754165103e-11,8.133355922669991e-08,], ];
        #[rustfmt::skip]
        let psi_after = array![ 47467.05706123715, 94934.11412247429, 118667.6426530929, 142401.1711837114, 189868.2282449486, 107947.0192373629 ];
        let cvals_after = array![0.8333333333333334, 0.7142857142857142, 1., 1., 1., 0.];
        let beta_after = array![1., 1., 1., 1.2, 1.4, 1.];

        let f = Lorenz63::default();
        let mut ida = Ida::new(f, array![0., 0., 0.], array![0., 0., 0.]);

        // Set preconditions:
        ida.ida_tn = 765020.5513257229;
        ida.ida_ns = 3;
        ida.ida_kk = 4;
        ida.ida_hh = 47467.05706123715;
        ida.ida_phi.assign(&phi_before);
        ida.ida_psi.assign(&psi_before);
        ida.ida_cvals.assign(&cvals_before);
        ida.ida_beta.assign(&beta_before);

        // Call the function under test
        ida.restore(saved_t);

        assert_nearly_eq!(ida.ida_tn, saved_t);
        assert_eq!(ida.ida_ns, 3);
        assert_eq!(ida.ida_kk, 4);
        assert_nearly_eq!(ida.ida_cvals, cvals_after, 1e-6);
        assert_nearly_eq!(ida.ida_beta, beta_after, 1e-6);
        assert_nearly_eq!(ida.ida_psi, psi_after, 1e-6);
        assert_nearly_eq!(ida.ida_phi, phi_after, 1e-6);
    }

    #[test]
    fn test_restore2() {
        let saved_t = 3623118336.24244;
        #[rustfmt::skip]
        let phi_before = array![ [5.716499633245077e-07,2.286601144610028e-12, 0.9999994283477499,], [-1.555846772013456e-07,-6.223394599091205e-13,1.555852991517385e-07,], [7.018252655941472e-08,2.807306512268244e-13,-7.01828076998538e-08,], [-4.56160628763917e-08,-1.824647796129851e-13,4.561624269904529e-08,], [5.593228676143622e-08,2.237297583983664e-13,-5.593253344183256e-08,], [-2.242367216194777e-10,-8.970915966733762e-16,2.242247401239887e-10,], ];
        #[rustfmt::skip]
        let psi_before = array![  857870592.1885694,   1286805888.282854,   1715741184.377139,   1930208832.424281,   2144676480.471424,    26020582.4876316];
        #[rustfmt::skip]
        let cvals_before = array![1., 1., 1., 1., 1., 1.];
        #[rustfmt::skip]
        let beta_before = array![1., 2., 3., 4.8, 7.199999999999999, 10.28571428571428];
        //--- IDARestore After: saved_t=   3623118336.24244 tn=   3623118336.24244 ns=1 kk=4
        #[rustfmt::skip]
        let phi_after = array![ [5.716499633245077e-07,2.286601144610028e-12, 0.9999994283477499,], [-7.779233860067279e-08,-3.111697299545603e-13,7.779264957586927e-08,], [2.339417551980491e-08,9.35768837422748e-14,-2.33942692332846e-08,], [-9.503346432581604e-09,-3.801349575270522e-14,9.503383895634436e-09,], [7.768373161310588e-09,3.107357755532867e-14,-7.768407422476745e-09,], [-2.242367216194777e-10,-8.970915966733762e-16,2.242247401239887e-10,], ];
        #[rustfmt::skip]
        let psi_after= array![  428935296.0942847,   857870592.1885694,   1072338240.235712,   1286805888.282854,   2144676480.471424,    26020582.4876316];
        #[rustfmt::skip]
        let cvals_after = array![ 0.5, 0.3333333333333333, 0.2083333333333333, 0.1388888888888889, 1., 1. ];
        #[rustfmt::skip]
        let beta_after = array![1., 2., 3., 4.8, 7.199999999999999, 10.28571428571428];

        let f = Lorenz63::default();
        let mut ida = Ida::new(f, array![0., 0., 0.], array![0., 0., 0.]);

        // Set preconditions:
        ida.ida_tn = 4480988928.431009;
        ida.ida_ns = 1;
        ida.ida_kk = 4;
        ida.ida_hh = 857870592.1885694;
        ida.ida_phi.assign(&phi_before);
        ida.ida_psi.assign(&psi_before);
        ida.ida_cvals.assign(&cvals_before);
        ida.ida_beta.assign(&beta_before);

        // Call the function under test
        ida.restore(saved_t);

        assert_nearly_eq!(ida.ida_tn, saved_t);
        assert_eq!(ida.ida_ns, 1);
        assert_eq!(ida.ida_kk, 4);
        assert_nearly_eq!(ida.ida_cvals, cvals_after, 1e-6);
        assert_nearly_eq!(ida.ida_beta, beta_after, 1e-6);
        assert_nearly_eq!(ida.ida_psi, psi_after, 1e-6);
        assert_nearly_eq!(ida.ida_phi, phi_after, 1e-6);
    }

    #[test]
    fn test_restore3() {
        let saved_t = 13638904.64873992;
        let phi_before = array![
            [
                0.0001523741818966069,
                6.095884948264652e-10,
                0.9998476252085154,
            ],
            [
                -1.964117218731689e-05,
                -7.858910051867137e-11,
                1.964125077907938e-05,
            ],
            [
                4.048658569496216e-06,
                1.620249912028008e-11,
                -4.048674765925692e-06,
            ],
            [
                -1.215165175266232e-06,
                -4.863765573523665e-12,
                1.21517004866448e-06,
            ],
            [
                4.909710408845208e-07,
                1.965778579990634e-12,
                -4.909729965008022e-07,
            ],
            [
                -2.529640523993838e-07,
                -1.012593011825966e-12,
                2.529650614751456e-07,
            ],
        ];
        let psi_before = array![
            1656116.685489699,
            2484175.028234549,
            3312233.370979399,
            4140291.713724249,
            5060356.538996303,
            5520388.951632331
        ];
        let cvals_before = array![1., 1., 1., 1., 1., 1.];
        let beta_before = array![1., 2., 3., 4., 4.864864864864866, 6.370656370656372];
        //--- IDARestore After: saved_t=  13638904.64873992 tn=  13638904.64873992 ns=1 kk=5
        let phi_after = array![
            [
                0.0001523741818966069,
                6.095884948264652e-10,
                0.9998476252085154,
            ],
            [
                -9.820586093658443e-06,
                -3.929455025933569e-11,
                9.820625389539692e-06,
            ],
            [
                1.349552856498739e-06,
                5.400833040093358e-12,
                -1.349558255308564e-06,
            ],
            [
                -3.037912938165579e-07,
                -1.215941393380916e-12,
                3.0379251216612e-07,
            ],
            [
                1.009218250707071e-07,
                4.040767081091857e-13,
                -1.009222270584982e-07,
            ],
            [
                -3.970769064935782e-08,
                -1.589464182199546e-13,
                3.970784904367437e-08,
            ],
        ];
        let psi_after = array![
            828058.3427448499,
            1656116.685489699,
            2484175.02823455,
            3404239.853506604,
            3864272.266142632,
            5520388.951632331,
        ];
        let cvals_after = array![
            0.5,
            0.3333333333333333,
            0.25,
            0.2055555555555555,
            0.156969696969697,
            1.,
        ];
        let beta_after = array![1., 2., 3., 4., 4.864864864864866, 6.370656370656372];

        let f = Lorenz63::default();
        let mut ida = Ida::new(f, array![0., 0., 0.], array![0., 0., 0.]);

        // Set preconditions:
        ida.ida_tn = 15295021.33422961;
        ida.ida_ns = 1;
        ida.ida_kk = 5;
        ida.ida_hh = 1656116.685489699;
        ida.ida_phi.assign(&phi_before);
        ida.ida_psi.assign(&psi_before);
        ida.ida_cvals.assign(&cvals_before);
        ida.ida_beta.assign(&beta_before);

        // Call the function under test
        ida.restore(saved_t);

        assert_nearly_eq!(ida.ida_tn, saved_t);
        assert_eq!(ida.ida_ns, 1);
        assert_eq!(ida.ida_kk, 5);
        assert_nearly_eq!(ida.ida_cvals, cvals_after, 1e-6);
        assert_nearly_eq!(ida.ida_beta, beta_after, 1e-6);
        assert_nearly_eq!(ida.ida_psi, psi_after, 1e-6);
        assert_nearly_eq!(ida.ida_phi, phi_after, 1e-6);
    }

    #[test]
    fn test_complete_step() {
        let err_k = 0.1022533962984153;
        let err_km1 = 0.3638660854770704;
        let ida_phi = array![
            [0.0000001057015204, 0.0000000000004228, 0.9999998942980568,],
            [-0.0000000330821964, -0.0000000000001323, 0.0000000330823287,],
            [0.0000000186752739, 0.0000000000000747, -0.0000000186753488,],
            [-0.0000000199565018, -0.0000000000000798, 0.0000000199565809,],
            [0.0000000012851942, 0.0000000000000051, -0.0000000012851948,],
            [-0.0000000002242367, -0.0000000000000009, 0.0000000002242247,],
        ];
        let ida_ee = array![-0.0000000051560075, -0.0000000000000206, 0.0000000051560285,];
        let ida_ewt = array![
            99894410.0897681862115860,
            999999.9999577193520963,
            9900.9911352019826154,
        ];
        let kk = 2;
        let kused = 2;
        let knew = 2;
        let phase = 1;
        let hh = 3774022770.1406540870666504;
        let hused = 4313148194.5176315307617188;
        let rr = 0.8750041964562566;
        let hmax_inv = 0.0000000000000000;
        let nst = 357;
        let maxord = 5;

        // Set preconditions:
        let f = Lorenz63::default();
        let mut ida = Ida::new(f, array![0., 0., 0.], array![0., 0., 0.]);
        ida.ida_nst = nst;
        ida.ida_kk = kk;
        ida.ida_hh = hh;
        ida.ida_rr = rr;
        ida.ida_kused = kused;
        ida.ida_hused = hused;
        ida.ida_knew = knew;
        ida.ida_maxord = maxord;
        ida.ida_phase = phase;
        ida.ida_hmax_inv = hmax_inv;
        ida.ida_ee.assign(&ida_ee);
        ida.ida_phi.assign(&ida_phi);
        ida.ida_ewt.assign(&ida_ewt);
        //ida.ida_Xvecs.assign(&ida_Xvecs);
        //ida.ida_Zvecs.assign(&ida_Zvecs);

        ida.complete_step(err_k, err_km1);

        let ida_phi = array![
            [0.0000000861385903, 0.0000000000003446, 0.9999999138610652,],
            [-0.0000000195629300, -0.0000000000000783, 0.0000000195630084,],
            [0.0000000135192664, 0.0000000000000541, -0.0000000135193203,],
            [-0.0000000051560075, -0.0000000000000206, 0.0000000051560285,],
            [0.0000000012851942, 0.0000000000000051, -0.0000000012851948,],
            [-0.0000000002242367, -0.0000000000000009, 0.0000000002242247,],
        ];
        let ida_ee = array![-0.0000000051560075, -0.0000000000000206, 0.0000000051560285,];
        let ida_ewt = array![
            99894410.0897681862115860,
            999999.9999577193520963,
            9900.9911352019826154,
        ];
        let kk = 2;
        let kused = 2;
        let knew = 2;
        let phase = 1;
        let hh = 3774022770.1406540870666504;
        let hused = 3774022770.1406540870666504;
        let rr = 1.6970448397793398;
        let hmax_inv = 0.0000000000000000;
        let nst = 358;
        let maxord = 5;

        assert_eq!(ida.ida_nst, nst);
        assert_eq!(ida.ida_kk, kk);
        assert_eq!(ida.ida_hh, hh);
        assert_nearly_eq!(ida.ida_rr, rr, 1e-6);
        assert_eq!(ida.ida_kused, kused);
        assert_eq!(ida.ida_hused, hused);
        assert_eq!(ida.ida_knew, knew);
        assert_eq!(ida.ida_maxord, maxord);
        assert_eq!(ida.ida_phase, phase);
        assert_eq!(ida.ida_hmax_inv, hmax_inv);
        assert_nearly_eq!(ida.ida_ee, ida_ee, 1e-6);
        assert_nearly_eq!(ida.ida_phi, ida_phi, 1e-6);
        assert_nearly_eq!(ida.ida_ewt, ida_ewt, 1e-6);
    }

    #[test]
    fn test_get_solution() {
        // --- IDAGetSolution Before:
        let t = 3623118336.24244;
        let hh = 857870592.1885694;
        let tn = 3623118336.24244;
        let kused = 4;
        let hused = 428935296.0942847;
        let ida_phi = array![
            [
                5.716499633245077e-07,
                2.286601144610028e-12,
                0.9999994283477499,
            ],
            [
                -7.779233860067279e-08,
                -3.111697299545603e-13,
                7.779264957586927e-08,
            ],
            [
                2.339417551980491e-08,
                9.35768837422748e-14,
                -2.33942692332846e-08,
            ],
            [
                -9.503346432581604e-09,
                -3.801349575270522e-14,
                9.503383895634436e-09,
            ],
            [
                7.768373161310588e-09,
                3.107357755532867e-14,
                -7.768407422476745e-09,
            ],
            [
                -2.242367216194777e-10,
                -8.970915966733762e-16,
                2.242247401239887e-10,
            ],
        ];
        let ida_psi = array![
            428935296.0942847,
            857870592.1885694,
            1072338240.235712,
            1286805888.282854,
            1501273536.329997,
            26020582.4876316,
        ];

        //--- IDAGetSolution After:
        let yret_expect = array![
            5.716499633245077e-07,
            2.286601144610028e-12,
            0.9999994283477499,
        ];
        let ypret_expect = array![
            -1.569167478317552e-16,
            -6.276676917262037e-22,
            1.569173718962504e-16,
        ];

        let f = Lorenz63::default();
        let mut ida = Ida::new(f, array![1., 2., 3.], array![4., 5., 6.]);
        //println!("{:#?}", i);

        ida.ida_hh = hh;
        ida.ida_tn = tn;
        ida.ida_kused = kused;
        ida.ida_hused = hused;
        ida.ida_phi.assign(&ida_phi);
        ida.ida_psi.assign(&ida_psi);

        let mut yret = Array::zeros((3));
        let mut ypret = Array::zeros((3));

        ida.get_solution(t, &mut yret, &mut ypret).unwrap();

        assert_nearly_eq!(yret, yret_expect, 1e-6);
        assert_nearly_eq!(ypret, ypret_expect, 1e-6);
    }
}
