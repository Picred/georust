use crate::coord_gen::get_next_coord;

mod coord_gen;

fn main() {
    let next = get_next_coord();
    println!("{:#?}", next)
}