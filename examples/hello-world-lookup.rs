extern crate squirrel;

fn main() {
    let mut vm = squirrel::Vm::new(1024);

    vm.push_root_table();
    vm.push_string("print").unwrap();
    vm.get(1).unwrap();
    vm.push_root_table();
    vm.push_string("Hello, World!\n").unwrap();
    vm.call(2).unwrap();
    vm.pop(3);
}

