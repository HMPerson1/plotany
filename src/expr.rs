use std::error;
use std::fmt;
use std::hash::BuildHasher;
use std::fmt::{Debug, Display, Formatter};
use std::collections::HashMap;

pub struct Equation(pub Expr, pub Expr);

pub enum Expr {
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Func(KnownFunc, Box<Expr>),
    Lit(f64),
    Var(String),
}

#[derive(Debug)]
pub enum KnownFunc {
    AbsoluteValue,
    Floor,
    Ceiling,
    Exponential,
    NaturalLogarithm,
    Sine,
    Cosine,
    Tangent,
    Secant,
    Cosecant,
    Cotangent,
    ArcSine,
    ArcCosine,
    ArcTangent,
    ArcSecant,
    ArcCosecant,
    ArcCotangent,
}

// do constant-folding at construction time
pub mod helper {
    use expr::*;
    pub fn add(e1: Expr, e2: Expr) -> Expr {
        match (e1, e2) {
            (Expr::Lit(l1), Expr::Lit(l2)) => Expr::Lit(l1 + l2),
            (e1, e2) => Expr::Add(Box::new(e1), Box::new(e2)),
        }
    }
    pub fn sub(e1: Expr, e2: Expr) -> Expr {
        match (e1, e2) {
            (Expr::Lit(l1), Expr::Lit(l2)) => Expr::Lit(l1 - l2),
            (e1, e2) => Expr::Sub(Box::new(e1), Box::new(e2)),
        }
    }
    pub fn mul(e1: Expr, e2: Expr) -> Expr {
        match (e1, e2) {
            (Expr::Lit(l1), Expr::Lit(l2)) => Expr::Lit(l1 * l2),
            (e1, e2) => Expr::Mul(Box::new(e1), Box::new(e2)),
        }
    }
    pub fn div(e1: Expr, e2: Expr) -> Expr {
        match (e1, e2) {
            (Expr::Lit(l1), Expr::Lit(l2)) => Expr::Lit(l1 / l2),
            (e1, e2) => Expr::Div(Box::new(e1), Box::new(e2)),
        }
    }
    pub fn pow(e1: Expr, e2: Expr) -> Expr {
        match (e1, e2) {
            (Expr::Lit(l1), Expr::Lit(l2)) => Expr::Lit(l1.powf(l2)),
            (e1, e2) => Expr::Pow(Box::new(e1), Box::new(e2)),
        }
    }
    pub fn neg(e: Expr) -> Expr {
        mul(e, lit(-1.0))
    }
    pub fn func(f: KnownFunc, e: Expr) -> Expr {
        match e {
            Expr::Lit(v) => Expr::Lit(f.eval(v)),
            e => Expr::Func(f, Box::new(e)),
        }
    }
    pub fn lit(v: f64) -> Expr {
        Expr::Lit(v)
    }
    pub fn var<S: Into<String>>(v: S) -> Expr {
        Expr::Var(v.into())
    }
}

#[derive(Debug)]
pub enum EvalError<'a> {
    UnknownVar(&'a str),
}

impl Equation {
    pub fn vars(&self) -> HashMap<&str, f64> {
        let mut r = self.0.vars();
        r.extend(self.1.vars());
        r
    }
    pub fn eval_diff<S: BuildHasher>(&self, env: &HashMap<&str, f64, S>) -> Result<f64, EvalError> {
        self.0.eval(env).and_then(|l| self.1.eval(env).map(|r| l-r))
    }
}

impl Expr {
    pub fn eval<S: BuildHasher>(&self, env: &HashMap<&str, f64, S>) -> Result<f64, EvalError> {
        use self::Expr::*;
        match *self {
            Add(ref a, ref b) => Ok(a.eval(env)? + b.eval(env)?),
            Sub(ref a, ref b) => Ok(a.eval(env)? - b.eval(env)?),
            Mul(ref a, ref b) => Ok(a.eval(env)? * b.eval(env)?),
            Div(ref a, ref b) => Ok(a.eval(env)? / b.eval(env)?),
            Pow(ref a, ref b) => Ok(a.eval(env)?.powf(b.eval(env)?)),
            Func(ref f, ref a) => Ok(f.eval(a.eval(env)?)),
            Lit(a) => Ok(a),
            Var(ref a) => env.get(a.as_str()).cloned().ok_or_else(|| EvalError::UnknownVar(a)),
        }
    }

    pub fn vars(&self) -> HashMap<&str, f64> {
        use self::Expr::*;
        match *self {
            Add(ref a, ref b) |
            Sub(ref a, ref b) |
            Mul(ref a, ref b) |
            Div(ref a, ref b) |
            Pow(ref a, ref b) => {
                let mut r = a.vars();
                r.extend(b.vars());
                r
            }
            Func(_, ref a) => a.vars(),
            Lit(_) => HashMap::new(),
            Var(ref a) => {
                let mut r = HashMap::new();
                r.insert(a.as_str(), 0.0);
                r
            }
        }
    }
}

impl KnownFunc {
    fn eval(&self, a: f64) -> f64 {
        use self::KnownFunc::*;
        match *self {
            AbsoluteValue => a.abs(),
            Floor => a.floor(),
            Ceiling => a.ceil(),
            Exponential => a.exp(),
            NaturalLogarithm => a.ln(),
            Sine => a.sin(),
            Cosine => a.cos(),
            Tangent => a.tan(),
            Secant => a.cos().recip(),
            Cosecant => a.sin().recip(),
            Cotangent => a.tan().recip(),
            ArcSine => a.asin(),
            ArcCosine => a.acos(),
            ArcTangent => a.atan(),
            ArcSecant => a.recip().acos(),
            ArcCosecant => a.recip().asin(),
            ArcCotangent => a.recip().atan(),
        }
    }
}

impl Debug for Equation {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        let Equation(ref l, ref r) = *self;
        write!(fmt, "{:?} = {:?}", l, r)
    }
}

impl Debug for Expr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        use self::Expr::*;
        match *self {
            Add(ref a, ref b) => write!(fmt, "({:?} + {:?})", a, b),
            Sub(ref a, ref b) => write!(fmt, "({:?} - {:?})", a, b),
            Mul(ref a, ref b) => write!(fmt, "({:?} * {:?})", a, b),
            Div(ref a, ref b) => write!(fmt, "({:?} / {:?})", a, b),
            Pow(ref a, ref b) => write!(fmt, "({:?} ^ {:?})", a, b),
            Func(ref f, ref a) => write!(fmt, "({:?}({:?}))", f, a),
            Lit(a) => write!(fmt, "{:?}", a),
            Var(ref a) => write!(fmt, "{}", a),
        }
    }
}

impl<'a> error::Error for EvalError<'a> {
    fn description(&self) -> &str {
        "unknown variable"
    }
}

impl<'a> Display for EvalError<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        use self::EvalError::*;
        write!(fmt, "failed to evaluate expression: ")?;
        match *self {
            UnknownVar(v) => write!(fmt, "unknown variable: {}", v),
        }
    }
}
