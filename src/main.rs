#![cfg_attr(feature = "cargo-clippy", warn(clippy,clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(missing_docs_in_private_items))]
#![feature(test)]
extern crate lalrpop_util;
extern crate ndarray;
extern crate gtk;
extern crate cairo;
extern crate test;
extern crate fnv;
extern crate meval;

// TODO: use meval
mod expr;
#[cfg_attr(feature = "cargo-clippy", allow(clippy,clippy_pedantic))]
mod expr_parser;
mod marching_squares;

use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use fnv::FnvHashMap;

use gtk::prelude::*;

fn main() {
    let r = main0();
    println!("{:?}", r);
}

macro_rules! get_objects_from_builder {
    ($b:ident, $($n:ident : $t:ty),*) => {
        $(
            let $n : $t = $b.get_object(stringify!($n))
                .expect(concat!("Failed to get `", stringify!($n), "`",
                                " from `", stringify!($b), "`"));
        )*
    }
}
macro_rules! cloning {
    ($($n:ident),+ => $body:expr) => {{
        $( let $n = $n.clone(); )+
        $body
    }}
}

fn main0() -> Result<(), Box<Error>> {
    // let eqn_string = "cos(x) + cos(y) = 1/2";
    // let eqn = expr_parser::parse_Equation(eqn_string);
    // let eqn = eqn.expect("aaaaaaa");
    // println!("{:?}", eqn);

    gtk::init().expect("failed to initialize GTK.");
    let builder = gtk::Builder::new_from_string(include_str!("layout.glade"));
    get_objects_from_builder!(builder,
                              window: gtk::Window,
                              drawing: gtk::DrawingArea,
                              entry_stack: gtk::Stack,
                              implicit_eqn_entry: gtk::Entry,
                              plot_btn: gtk::Button,
                              info_bar: gtk::InfoBar,
                              info_label: gtk::Label,
                              info_bar_revealer: gtk::Revealer);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let eq: Rc<RefCell<Option<expr::Equation>>> = Default::default();

    plot_btn.connect_clicked(
        cloning!(eq, implicit_eqn_entry, info_bar_revealer, info_label, drawing => move |_| {
            // println!("{:?}", implicit_eqn_entry.get_text());
            drawing.queue_draw();
            *eq.borrow_mut() = {
                let new_eq =
                    implicit_eqn_entry.get_text().ok_or("internal: no text")
                    .and_then(|s| expr_parser::parse_Equation(&s).map_err(|_| "parse error"))
                    .and_then(|ne|
                              if ne.vars().keys().all(|v| v == &"x" || v == &"y") {
                                  Ok(ne)
                              } else {
                                  Err("free variables")
                              }
                    );
                match new_eq {
                    Ok(_) => info_bar_revealer.set_reveal_child(false),
                    Err(e) => {
                        info_label.set_text(e);
                        info_bar_revealer.set_reveal_child(true);
                    }
                }
                new_eq.ok()
            }
        }));

    info_bar.connect_response(cloning!(info_bar_revealer => move |_, _| {
        info_bar_revealer.set_reveal_child(false);
    }));

    drawing.connect_draw(cloning!(eq => move |_, ctx| {
        let mut map = HashMap::<&str, f64>::with_capacity(2);
        marching_squares::marching_squares(ctx,
                                           |x, y| {
                                               map.insert("x", x);
                                               map.insert("y", y);
                                               if let Some(ref eq) = *eq.borrow() {
                                                   eq.eval_diff(&map).expect("eval in mar_sq")
                                               } else {
                                                   0.0
                                               }
                                           },
                                           &(-10.0..10.0),
                                           256,
                                           &(-10.0..10.0),
                                           256);
        Inhibit(false)
    }));

    window.show_all();
    // `drawing`'s size_request is set in `layout.glade`
    // but for some reason that causes weirdness with resizing the infobar
    // so we let the size allocation process thing happen once with
    // the size request set so that when we start `drawing` is a sensible size,
    // then we unset them here so resizing the infobar works
    drawing.set_size_request(-1, -1);
    gtk::main();

    Ok(())
}

#[test]
fn parser() {
    assert!(expr_parser::parse_Expr("-a^-b").is_err());
    assert!(expr_parser::parse_Expr("-a^(-b)").is_ok());
    assert!(expr_parser::parse_Expr("a * -b").is_ok());
    assert!(expr_parser::parse_Expr(
        "abs(floor(ceil(exp(ln(sin(cos(tan(sec(csc(cot(arcsin(arccos(arctan(arcsec(\
         arccsc(arccot(1)))))))))))))))))"
    ).is_ok());
    assert!(expr_parser::parse_Equation("x=22").is_ok());
    assert!(expr_parser::parse_Equation("x=((22)").is_err());
    assert!(expr_parser::parse_Equation("cos(x) + cos(y) = 1/2").is_ok());
}

#[test]
fn expr_vars() {
    use expr::*;
    fn ck_vars(e: Expr, i: &[&str]) -> bool {
        i.into_iter().collect::<HashSet<_>>() == e.vars().keys().collect()
    }
    use expr::helper::*;
    assert!(ck_vars(func(KnownFunc::Sine, lit(1.0)), &[]));
    assert!(ck_vars(func(KnownFunc::Sine, mul(var("eee"), var("qqq"))),
                    &["eee", "qqq"]));
    let abc: Vec<String> = (0x61..0x7A).map(|c| char::from(c).to_string()).collect();
    let abc: Vec<&str> = abc.iter().map(|s| s.as_str()).collect();
    assert!(ck_vars(abc.iter().cloned().map(var).fold(lit(0.0), add),
                    abc.as_slice()));
}


const BENCH_EXPR: &'static str = "abs(sin(x + 1) * (x^2 + x + 1))";
#[bench]
fn meval_expr_eval_bench(b: &mut test::Bencher) {
    let e: meval::Expr = BENCH_EXPR.parse().unwrap();
    let f = e.bind("x").unwrap();
    b.iter(move || {
        // 143ns +/- 3
        f(1.0);
    })
}
#[bench]
fn my_expr_eval_fnvhash_bench(b: &mut test::Bencher) {
    let e = expr_parser::parse_Expr(BENCH_EXPR).unwrap();
    let mut map = FnvHashMap::<&str, f64>::with_capacity_and_hasher(1, Default::default());
    b.iter(move || {
        // 109ns +/- 1
        map.insert("x", 1.0);
        e.eval(&map);
    })
}
#[bench]
fn my_expr_eval_siphash_bench(b: &mut test::Bencher) {
    let e = expr_parser::parse_Expr(BENCH_EXPR).unwrap();
    let mut map = HashMap::<&str, f64>::with_capacity(1);
    b.iter(move || {
        // 193ns +/- 4
        map.insert("x", 1.0);
        e.eval(&map);
    })
}

const BENCH_EXPR_2: &'static str = "abs(sin(x + 1) * (x^2 + x + 1)) + abs(sin(y + 1) * (y^2 + y + 1))";
// const BENCH_EXPR_2: &'static str = "x*x*x*x*x*y";
#[bench]
fn meval_expr_eval2_bench(b: &mut test::Bencher) {
    let e: meval::Expr = BENCH_EXPR_2.parse().unwrap();
    let f = e.bind2("x", "y").unwrap();
    b.iter(move || {
        f(1.0, 3.0);
    })
}
#[bench]
fn my_expr_eval2_fnvhash_bench(b: &mut test::Bencher) {
    let e = expr_parser::parse_Expr(BENCH_EXPR_2).unwrap();
    let mut map = FnvHashMap::<&str, f64>::with_capacity_and_hasher(2, Default::default());
    b.iter(move || {
        map.insert("x", 1.0);
        map.insert("y", 3.0);
        e.eval(&map);
    })
}

#[bench]
fn my_expr_eval_bench(b: &mut test::Bencher) {
    let e = expr_parser::parse_Expr("cos(1.5*x) + cos(y) - 1").unwrap();
    let mut map = FnvHashMap::<&str, f64>::with_capacity_and_hasher(2, Default::default());
    map.insert("x", 2.0);
    map.insert("y", 3.0);
    b.iter(move || {
        for _ in 0..1000 {
            e.eval(&map);
        }
    })
}

#[bench]
fn fnvhashget(b: &mut test::Bencher) {
    let mut map = FnvHashMap::<&str, f64>::with_capacity_and_hasher(1, Default::default());
    map.insert("0", 1.0);
    b.iter(|| for _ in 0..1000 { map.get("0"); });
}
#[bench]
fn fnvhashinsert(b: &mut test::Bencher) {
    let mut map = FnvHashMap::<&str, f64>::with_capacity_and_hasher(1, Default::default());
    b.iter(|| for _ in 0..1000 { map.insert("0", 1.0); });
}
#[bench]
fn vec_write(b: &mut test::Bencher) {
    let mut v = vec![0];
    b.iter(|| for _ in 0..1000 { unsafe { *v.get_unchecked_mut(0) = test::black_box(1); } });
}
#[bench]
fn vec_read(b: &mut test::Bencher) {
    let mut v = vec![0];
    b.iter(|| for _ in 0..1000 { unsafe { test::black_box(v.get_unchecked(0)); } });
}

#[bench]
fn plot_bench(b: &mut test::Bencher) {
    let surf = cairo::ImageSurface::create(cairo::Format::Rgb24, 500, 500);
    let ctx = cairo::Context::new(&surf);

    let eq = expr_parser::parse_Equation("x^2 + y^2 = 5^2").unwrap();

    b.iter(move || {
        // 6.383ms +/- 0.322
        let mut map = FnvHashMap::<&str, f64>::with_capacity_and_hasher(2, Default::default());
        marching_squares::marching_squares(&ctx,
                                           |x, y| {
                                               map.insert("x", x);
                                               map.insert("y", y);
                                               eq.eval_diff(&map).expect("eval in mar_sq")
                                           },
                                           &(-10.0..10.0),
                                           256,
                                           &(-10.0..10.0),
                                           256);
    });
}
