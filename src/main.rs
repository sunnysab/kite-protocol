trait Ops2 {
    fn print() {}
}
trait Ops1: Ops2 {
    fn will() {}
}

struct S;

impl Ops2 for S {}
impl Ops1 for S {}

fn main() {
    S::print();
}
