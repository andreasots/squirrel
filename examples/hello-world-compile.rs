extern crate squirrel;

fn main() {
    let mut vm = squirrel::Vm::new(1024);

    vm.compile(&mut "print(\"Hello, World!\\n\");".as_bytes(), "hello.nut").unwrap();
    vm.push_root_table();
    vm.call(1).unwrap();
    vm.pop(2);
}
