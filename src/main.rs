#![cfg_attr(feature = "cargo-clippy", warn(clippy,clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(missing_docs_in_private_items))]
// #![feature(test)]
// extern crate test;
extern crate ndarray;
extern crate gtk;
extern crate cairo;
extern crate fnv;
#[macro_use]
extern crate lalrpop_util;

mod expr;
#[cfg_attr(feature = "cargo-clippy", allow(clippy,clippy_pedantic))]
lalrpop_mod!(expr_parser);
mod marching_squares;

use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Range;

use gtk::prelude::*;

fn main() {
    let r = main0();
    println!("{:?}", r);
}

macro_rules! get_objects_from_builder {
    ($b:ident, $($n:ident : $t:ty),*) => {
        $(
            let $n : $t = $b.object(stringify!($n))
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
    let builder = gtk::Builder::from_string(include_str!("layout.glade"));
    get_objects_from_builder!(builder,
                              window: gtk::Window,
                              drawing: gtk::DrawingArea,
                              entry_stack: gtk::Stack,
                              implicit_eqn_entry: gtk::Entry,
                              x_min_entry: gtk::SpinButton,
                              x_max_entry: gtk::SpinButton,
                              y_min_entry: gtk::SpinButton,
                              y_max_entry: gtk::SpinButton,
                              plot_btn: gtk::Button,
                              info_bar: gtk::InfoBar,
                              info_label: gtk::Label,
                              info_bar_revealer: gtk::Revealer);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let eq: Rc<RefCell<Option<Box<FnMut(f64, f64) -> f64>>>> = Default::default();
    let x_range: Rc<RefCell<Range<f64>>> = Rc::new(RefCell::new(-10.0..10.0));
    let y_range: Rc<RefCell<Range<f64>>> = Rc::new(RefCell::new(-10.0..10.0));

    {
        use std::f64::MAX;
        x_min_entry.set_range(-MAX, MAX);
        x_min_entry.set_value(-10.0);
        x_max_entry.set_range(-MAX, MAX);
        x_max_entry.set_value(10.0);
        y_min_entry.set_range(-MAX, MAX);
        y_min_entry.set_value(-10.0);
        y_max_entry.set_range(-MAX, MAX);
        y_max_entry.set_value(10.0);
    }

    plot_btn.connect_clicked(
        cloning!(eq, x_range, y_range, x_min_entry, x_max_entry, y_min_entry, y_max_entry, implicit_eqn_entry, info_bar_revealer, info_label, drawing => move |_| {
            // println!("{:?}", implicit_eqn_entry.get_text());
            drawing.queue_draw();
            *eq.borrow_mut() = {
                let new_eq =
                    expr_parser::EquationParser::new().parse(implicit_eqn_entry.text().as_str()).map_err(|_| "parse error")
                    .and_then(|ne| {
                              let cne = ne.to_diff().compile();
                              if cne.vars().iter().all(|v| v == &"x" || v == &"y") {
                                  Ok(cne.bind2("x","y"))
                              } else {
                                  Err("free variables")
                              }
                    });
                match new_eq {
                    Ok(ne) => {
                        info_bar_revealer.set_reveal_child(false);
                        Some(ne)
                    }
                    Err(e) => {
                        info_label.set_text(e);
                        info_bar_revealer.set_reveal_child(true);
                        None
                    }
                }
            };
            *x_range.borrow_mut() = x_min_entry.value() .. x_max_entry.value();
            *y_range.borrow_mut() = y_min_entry.value() .. y_max_entry.value();
        }));

    info_bar.connect_response(cloning!(info_bar_revealer => move |_, _| {
        info_bar_revealer.set_reveal_child(false);
    }));


    drawing.connect_draw(cloning!(eq, x_range, y_range => move |_, ctx| {
        let mut eq = eq.borrow_mut();
        let mut const_zero = |_,_| 0.0;
        let f = if let Some(ref mut eq) = *eq {
            &mut **eq
        } else {
            &mut const_zero
        };
        marching_squares::marching_squares(ctx,
                                           f,
                                           &*x_range.borrow(),
                                           256,
                                           &*y_range.borrow(),
                                           256);
        Inhibit(false)
    }));

    window.show();
    // `drawing`'s size_request is set in `layout.glade`
    // but for some reason that causes weirdness with resizing the infobar
    // so we let the size allocation process thing happen once with
    // the size request set so that when we start `drawing` is a sensible size,
    // then we unset them here so resizing the infobar works
    drawing.set_size_request(-1, -1);
    gtk::main();

    Ok(())
}

// #[test]
// fn parser() {
//     assert!(expr_parser::parse_Expr("-a^-b").is_err());
//     assert!(expr_parser::parse_Expr("-a^(-b)").is_ok());
//     assert!(expr_parser::parse_Expr("a * -b").is_ok());
//     assert!(expr_parser::parse_Expr("5b").is_ok());
//     assert!(expr_parser::parse_Expr(
//         "abs(floor(ceil(exp(ln(sin(cos(tan(sec(csc(cot(arcsin(arccos(arctan(arcsec(\
//          arccsc(arccot(1)))))))))))))))))"
//     ).is_ok());
//     assert!(expr_parser::parse_Equation("x=22").is_ok());
//     assert!(expr_parser::parse_Equation("x=((22)").is_err());
//     assert!(expr_parser::parse_Equation("cos(x) + cos(y) = 1/2").is_ok());
// }

// #[bench]
// fn my_comp_expr_eval_bench(b: &mut test::Bencher) {
//     let mut e = expr_parser::parse_Expr("cos(1.5*x) + cos(y) - 1")
//         .unwrap()
//         .compile()
//         .bind2("x", "y");
//     b.iter(move || {
//         for _ in 0..1000 {
//             // 80.9ns +/- 1.0
//             e(2.0, 3.0);
//         }
//     })
// }

// #[bench]
// fn plot_bench(b: &mut test::Bencher) {
//     let surf = cairo::ImageSurface::create(cairo::Format::Rgb24, 500, 500);
//     let ctx = cairo::Context::new(&surf);

//     let eq = expr_parser::parse_Equation("x^2 + y^2 = 5^2").unwrap();
//     let mut f = eq.to_diff().compile().bind2("x", "z");

//     b.iter(move || {
//         // 4.138ms +/- 0.223
//         marching_squares::marching_squares(&ctx, &mut *f, &(-10.0..10.0), 256, &(-10.0..10.0), 256);
//     });
// }
