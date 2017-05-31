// #![cfg_attr(feature = "cargo-clippy", warn(clippy,clippy_pedantic))]
// #![cfg_attr(feature = "cargo-clippy", allow(missing_docs_in_private_items))]
extern crate lalrpop_util;
extern crate ndarray;
extern crate gtk;
extern crate cairo;

mod expr;
#[cfg_attr(feature = "cargo-clippy", allow(clippy,clippy_pedantic))]
mod expr_parser;
mod marching_squares;

use std::error::Error;
use std::collections::HashMap;

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
    let eqn_string = "cos(x) + cos(y) = 1/2";
    let eqn = expr_parser::parse_Equation(eqn_string);
    let eqn = eqn.expect("aaaaaaa");
    println!("{:?}", eqn);

    gtk::init().expect("failed to initialize GTK.");
    let builder = gtk::Builder::new_from_string(include_str!("layout.glade"));
    get_objects_from_builder!(builder,
                              window: gtk::Window,
                              drawing: gtk::DrawingArea,
                              entry_stack: gtk::Stack,
                              implicit_eqn_entry: gtk::Entry,
                              plot_btn: gtk::Button);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    drawing.connect_draw(move |_, ctx| {
        let mut map = HashMap::<&str, f64>::with_capacity(2);
        marching_squares::marching_squares(ctx,
                                           |x, y| {
            map.clear();
            map.insert("x", x);
            map.insert("y", y);
            let lv = eqn.0.eval(&map).unwrap();
            let rv = eqn.1.eval(&map).unwrap();
            lv - rv
        },
                                           &(-10.0..10.0),
                                           64,
                                           &(-10.0..10.0),
                                           64);
        Inhibit(false)
    });

    window.show_all();
    gtk::main();

    Ok(())
}

#[test]
fn parser() {
    assert!(expr_parser::parse_Equation("x=22").is_ok());
    assert!(expr_parser::parse_Equation("x=(22)").is_ok());
    assert!(expr_parser::parse_Equation("x=((((22))))").is_ok());
    assert!(expr_parser::parse_Equation("x=((22)").is_err());
}
