#[macro_use]
extern crate squirrel;

closure! {
    fn hello(_) {
        println!("Hello, World!");
        squirrel::ClosureResult::NoReturnValue
    }
}

fn main() {
    let mut vm = squirrel::Vm::new(1024);

    vm.new_closure(hello, 0);
    vm.push_root_table();
    vm.call(1).unwrap();
    vm.pop(2);
}

