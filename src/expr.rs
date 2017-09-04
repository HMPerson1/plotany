use std::error;
use std::fmt;
use std::hash::BuildHasher;
use std::fmt::{Debug, Display, Formatter};
use std::collections::HashMap;

use fnv::FnvHashMap;

pub struct Equation(pub Expr, pub Expr);

#[derive(Debug)]
pub struct Expr(BaseExpr<String>);

enum BaseExpr<V> {
    Add(Box<BaseExpr<V>>, Box<BaseExpr<V>>),
    Sub(Box<BaseExpr<V>>, Box<BaseExpr<V>>),
    Mul(Box<BaseExpr<V>>, Box<BaseExpr<V>>),
    Div(Box<BaseExpr<V>>, Box<BaseExpr<V>>),
    Pow(Box<BaseExpr<V>>, Box<BaseExpr<V>>),
    Func(KnownFunc, Box<BaseExpr<V>>),
    Lit(f64),
    Var(V),
}

/// invariant: all BaseExpr::Var(usize) in `expr` and values in `varmap`
/// are valid indices into `varstore`
#[derive(Debug)]
pub struct CompiledExpr {
    expr: BaseExpr<usize>,
    varmap: FnvHashMap<String, usize>,
    varstore: Vec<f64>,
}

#[derive(Debug, Copy, Clone)]
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
        match (e1.0, e2.0) {
            (BaseExpr::Lit(l1), BaseExpr::Lit(l2)) => Expr(BaseExpr::Lit(l1 + l2)),
            (e1, e2) => Expr(BaseExpr::Add(Box::new(e1), Box::new(e2))),
        }
    }
    pub fn sub(e1: Expr, e2: Expr) -> Expr {
        match (e1.0, e2.0) {
            (BaseExpr::Lit(l1), BaseExpr::Lit(l2)) => Expr(BaseExpr::Lit(l1 - l2)),
            (e1, e2) => Expr(BaseExpr::Sub(Box::new(e1), Box::new(e2))),
        }
    }
    pub fn mul(e1: Expr, e2: Expr) -> Expr {
        match (e1.0, e2.0) {
            (BaseExpr::Lit(l1), BaseExpr::Lit(l2)) => Expr(BaseExpr::Lit(l1 * l2)),
            (e1, e2) => Expr(BaseExpr::Mul(Box::new(e1), Box::new(e2))),
        }
    }
    pub fn div(e1: Expr, e2: Expr) -> Expr {
        match (e1.0, e2.0) {
            (BaseExpr::Lit(l1), BaseExpr::Lit(l2)) => Expr(BaseExpr::Lit(l1 / l2)),
            (e1, e2) => Expr(BaseExpr::Div(Box::new(e1), Box::new(e2))),
        }
    }
    pub fn pow(e1: Expr, e2: Expr) -> Expr {
        match (e1.0, e2.0) {
            (BaseExpr::Lit(l1), BaseExpr::Lit(l2)) => Expr(BaseExpr::Lit(l1.powf(l2))),
            (e1, e2) => Expr(BaseExpr::Pow(Box::new(e1), Box::new(e2))),
        }
    }
    pub fn neg(e: Expr) -> Expr {
        mul(e, lit(-1.0))
    }
    pub fn func(f: KnownFunc, e: Expr) -> Expr {
        match e.0 {
            BaseExpr::Lit(v) => Expr(BaseExpr::Lit(f.eval(v))),
            e => Expr(BaseExpr::Func(f, Box::new(e))),
        }
    }
    pub fn lit(v: f64) -> Expr {
        Expr(BaseExpr::Lit(v))
    }
    pub fn var<S: Into<String>>(v: S) -> Expr {
        Expr(BaseExpr::Var(v.into()))
    }
}

#[derive(Debug)]
pub enum EvalError<'a> {
    UnknownVar(&'a str),
}

impl Equation {
    // pub fn vars(&self) -> HashMap<&str, f64> {
    //     let mut r = self.0.vars();
    //     r.extend(self.1.vars());
    //     r
    // }
    pub fn eval_diff<S: BuildHasher>(&self, env: &HashMap<&str, f64, S>) -> Result<f64, EvalError> {
        self.0.eval(env).and_then(|l| self.1.eval(env).map(|r| l - r))
    }

    pub fn to_diff(self) -> Expr {
        helper::sub(self.0, self.1)
    }
}

impl Expr {
    pub fn eval<S: BuildHasher>(&self, env: &HashMap<&str, f64, S>) -> Result<f64, EvalError> {
        self.0.eval(env)
    }
    // pub fn vars(&self) -> HashMap<&str, f64> {
    //     use self::BaseExpr::*;
    //     match self.0 {
    //         Add(ref a, ref b) |
    //         Sub(ref a, ref b) |
    //         Mul(ref a, ref b) |
    //         Div(ref a, ref b) |
    //         Pow(ref a, ref b) => {
    //             let mut r = Expr(**a).vars();
    //             r.extend(Expr(**b).vars());
    //             r
    //         }
    //         Func(_, ref a) => Expr(**a).vars(),
    //         Lit(_) => HashMap::new(),
    //         Var(ref a) => {
    //             let mut r = HashMap::new();
    //             r.insert(a.as_str(), 0.0);
    //             r
    //         }
    //     }
    // }

    pub fn compile(self) -> CompiledExpr {
        let mut map = FnvHashMap::default();
        let mut store = Vec::new();
        let e = self.vecvar0(&mut map, &mut store);
        map.shrink_to_fit();
        store.shrink_to_fit();
        CompiledExpr {
            expr: e,
            varmap: map,
            varstore: store,
        }
    }

    fn vecvar0(self,
               varmap: &mut FnvHashMap<String, usize>,
               varstore: &mut Vec<f64>)
               -> BaseExpr<usize> {
        use self::BaseExpr::*;
        match self.0 {
            Add(a, b) => {
                let a = Expr(*a).vecvar0(varmap, varstore);
                let b = Expr(*b).vecvar0(varmap, varstore);
                Add(Box::new(a), Box::new(b))
            }
            Sub(a, b) => {
                let a = Expr(*a).vecvar0(varmap, varstore);
                let b = Expr(*b).vecvar0(varmap, varstore);
                Sub(Box::new(a), Box::new(b))
            }
            Mul(a, b) => {
                let a = Expr(*a).vecvar0(varmap, varstore);
                let b = Expr(*b).vecvar0(varmap, varstore);
                Mul(Box::new(a), Box::new(b))
            }
            Div(a, b) => {
                let a = Expr(*a).vecvar0(varmap, varstore);
                let b = Expr(*b).vecvar0(varmap, varstore);
                Div(Box::new(a), Box::new(b))
            }
            Pow(a, b) => {
                let a = Expr(*a).vecvar0(varmap, varstore);
                let b = Expr(*b).vecvar0(varmap, varstore);
                Pow(Box::new(a), Box::new(b))
            }
            Func(f, a) => {
                let a = Expr(*a).vecvar0(varmap, varstore);
                Func(f, Box::new(a))
            }
            Lit(l) => Lit(l),
            Var(a) => {
                use std::collections::hash_map::Entry;
                Var(match varmap.entry(a) {
                    Entry::Occupied(entry) => *entry.get(),
                    Entry::Vacant(entry) => {
                        let n = varstore.len();
                        varstore.push(0.0);
                        entry.insert(n);
                        n
                    },
                })
            }
        }
    }
}

impl BaseExpr<usize> {
    fn eval(&self, env: &[f64]) -> f64 {
        use self::BaseExpr::*;
        match *self {
            Add(ref a, ref b) => a.eval(env) + b.eval(env),
            Sub(ref a, ref b) => a.eval(env) - b.eval(env),
            Mul(ref a, ref b) => a.eval(env) * b.eval(env),
            Div(ref a, ref b) => a.eval(env) / b.eval(env),
            Pow(ref a, ref b) => a.eval(env).powf(b.eval(env)),
            Func(ref f, ref a) => f.eval(a.eval(env)),
            Lit(a) => a,
            Var(a) => env[a],
        }
    }
}

impl BaseExpr<String> {
    pub fn eval<S: BuildHasher>(&self, env: &HashMap<&str, f64, S>) -> Result<f64, EvalError> {
        use self::BaseExpr::*;
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
}

impl CompiledExpr {
    pub fn vars(&self) -> Vec<&str>{
        self.varmap.keys().map(|s| s.as_str()).collect::<Vec<_>>()
    }
    pub fn set_var(&mut self, name: &str, value: f64) {
        self.varmap.get(name).cloned().map(|i| self.varstore[i] = value);
    }
    pub fn eval(&self) -> f64 {
        self.expr.eval(&self.varstore)
    }
    pub fn bind2(self, v1: &str, v2: &str) -> Box<FnMut(f64, f64) -> f64> {
        let CompiledExpr { expr, varmap, mut varstore } = self;
        let v1 = varmap.get(v1).cloned();
        let v2 = varmap.get(v2).cloned();
        Box::new(move |x, y| {
            v1.map(|v1| varstore[v1] = x);
            v2.map(|v2| varstore[v2] = y);
            expr.eval(&varstore)
        })
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

impl<V: Debug> Debug for BaseExpr<V> {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        use self::BaseExpr::*;
        match *self {
            Add(ref a, ref b) => write!(fmt, "({:?} + {:?})", a, b),
            Sub(ref a, ref b) => write!(fmt, "({:?} - {:?})", a, b),
            Mul(ref a, ref b) => write!(fmt, "({:?} * {:?})", a, b),
            Div(ref a, ref b) => write!(fmt, "({:?} / {:?})", a, b),
            Pow(ref a, ref b) => write!(fmt, "({:?} ^ {:?})", a, b),
            Func(ref f, ref a) => write!(fmt, "({:?}({:?}))", f, a),
            Lit(a) => write!(fmt, "{:?}", a),
            Var(ref a) => write!(fmt, "Var({:?})", a),
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
