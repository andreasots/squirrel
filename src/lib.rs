extern crate squirrel_sys;

use std::io::prelude::*;
use std::ffi::CString;

use squirrel_sys::*;

pub mod ffi {
    pub use ::squirrel_sys::*;
}

#[derive(Debug)]
pub enum Error {
    Nul(std::ffi::NulError),
    Str(String),
    Vm,
    Compiler { desc: String, file: String, line: SQInteger, col: SQInteger },
    Exception(String),
}

impl From<std::ffi::NulError> for Error {
    fn from(e: std::ffi::NulError) -> Error {
        Error::Nul(e)
    }
}

extern "C" fn readfunc<R: Read>(reader: SQUserPointer) -> SQInteger {
    let reader: &mut R = unsafe { &mut *(reader as *mut R) };
    let mut buf = [0u8];
    reader.read(&mut buf).map(|_| buf[0] as SQInteger).unwrap_or(0)
}

#[repr(C)]
pub struct VmData {
    print_fn: extern "C" fn(data: *mut VmData, string: *const SQChar),
    error_fn: extern "C" fn(data: *mut VmData, string: *const SQChar),

    print_callback: Box<FnMut(&str)>,
    error_callback: Box<FnMut(&str)>,
    last_error: Option<Error>,
}

extern "C" fn print_fn_rust(data: *mut VmData, string: *const SQChar) {
    let ref mut callback = unsafe { &mut *data }.print_callback;
    let string = String::from_utf8_lossy(unsafe { std::ffi::CStr::from_ptr(string) }.to_bytes());
    callback(&string)
}

extern "C" fn error_fn_rust(data: *mut VmData, string: *const SQChar) {
    let ref mut callback = unsafe { &mut *data }.error_callback;
    let string = String::from_utf8_lossy(unsafe { std::ffi::CStr::from_ptr(string) }.to_bytes());
    callback(&string)
}

#[derive(Debug)]
pub enum Object {
    Array,
    Bool(bool),
    Class,
    Closure,
    Float(SQFloat),
    FuncProto,
    Generator,
    Instance,
    Integer(SQInteger),
    NativeClosure,
    Null,
    Outer,
    String(String),
    Table,
    Thread,
    UserData { p: SQUserPointer, typetag: SQUserPointer },
    UserPointer(SQUserPointer),
    WeakRef,
}

fn convert_object(vm: HSQUIRRELVM, idx: SQInteger) -> Result<Object, Error> {
    match unsafe { sq_gettype(vm, idx) } {
        OT_ARRAY => Ok(Object::Array),
        OT_BOOL => {
            let mut v = 0;
            if unsafe { sq_getbool(vm, idx, &mut v) } >= 0 {
                Ok(Object::Bool(v == 1))
            } else {
                Err(Error::Vm)
            }
        },
        OT_CLASS => Ok(Object::Class),
        OT_CLOSURE => Ok(Object::Closure),
        OT_FLOAT => {
            let mut v = 0.0;
            if unsafe { sq_getfloat(vm, idx, &mut v) } >= 0 {
                Ok(Object::Float(v))
            } else {
                Err(Error::Vm)
            }
        },
        OT_FUNCPROTO => Ok(Object::FuncProto),
        OT_GENERATOR => Ok(Object::Generator),
        OT_INSTANCE => Ok(Object::Instance),
        OT_INTEGER => {
            let mut v = 0;
            if unsafe { sq_getinteger(vm, idx, &mut v) } >= 0 {
                Ok(Object::Integer(v))
            } else {
                Err(Error::Vm)
            }
        },
        OT_NATIVECLOSURE => Ok(Object::NativeClosure),
        OT_NULL => Ok(Object::Null),
        OT_OUTER => Ok(Object::Outer),
        OT_STRING => {
            let mut v = 0 as *const SQChar;
            if unsafe { sq_getstring(vm, idx, &mut v) } >= 0 {
                Ok(Object::String(String::from_utf8_lossy(unsafe { std::ffi::CStr::from_ptr(v) }.to_bytes()).to_string()))
            } else {
                Err(Error::Vm)
            }
        },
        OT_TABLE => Ok(Object::Table),
        OT_THREAD => Ok(Object::Thread),
        OT_USERDATA => {
            let mut p = 0 as SQUserPointer;
            let mut tag = 0 as SQUserPointer;
            if unsafe { sq_getuserdata(vm, idx, &mut p, &mut tag) } >= 0 {
                Ok(Object::UserData { p: p, typetag: tag })
            } else {
                Err(Error::Vm)
            }
        },
        OT_USERPOINTER => {
            let mut p = 0 as SQUserPointer;
            if unsafe { sq_getuserpointer(vm, idx, &mut p) } >= 0 {
                Ok(Object::UserPointer(p))
            } else {
                Err(Error::Vm)
            }
        },
        OT_WEAKREF => Ok(Object::WeakRef),
        type_ => Err(Error::Str(format!("VM stack contains an unknown type {:#x} at offset {}", type_, idx))),
    }
}


pub enum ClosureResult {
    /// Function didn't return a value
    NoReturnValue = 0,
    /// Top of the stack contains the return value
    ReturnValue = 1,
    /// Throw a runtime error
    Error = -1,
}

#[macro_export]
macro_rules! closure {
    (fn $name:ident ($pat:pat) $body:block) => (extern fn $name(vm: $crate::ffi::HSQUIRRELVM) -> $crate::ffi::SQInteger {
        let mut vm = unsafe { $crate::Vm::from_raw(vm) };
        let res: $crate::ClosureResult = {
            let $pat = &mut vm;
            $body
        };
        std::mem::forget(vm);
        return res as $crate::ffi::SQInteger;
    });
}

extern "C" fn compiler_error_handler(vm: HSQUIRRELVM,
                                     desc: *const SQChar,
                                     file: *const SQChar, line: SQInteger, col: SQInteger) {
    let data = unsafe { &mut *(sq_getforeignptr(vm) as *mut VmData) };
    data.last_error = Some(Error::Compiler {
        desc: String::from_utf8_lossy(unsafe { std::ffi::CStr::from_ptr(desc) }.to_bytes()).into_owned(),
        file: String::from_utf8_lossy(unsafe { std::ffi::CStr::from_ptr(file) }.to_bytes()).into_owned(),
        line: line,
        col: col,
    });
}

closure! {
    fn exception_handler(vm) {
        vm.1.last_error = Some(match vm.to_string(2).and_then(|()| convert_object(vm.0, 3)) {
            Ok(Object::String(string)) => Error::Exception(string),
            Ok(_) => unreachable!(),
            Err(e) => e,
        });
        ClosureResult::NoReturnValue
    }
}

pub struct Vm(HSQUIRRELVM, Box<VmData>);

impl Vm {
    /// Create a new virtual machine instance with the specified initial stack size.
    pub fn new(initial_stack_size: SQInteger) -> Vm {
        let vm = unsafe { sq_open(initial_stack_size) };
        let data = Box::new(VmData {
            print_fn: print_fn_rust,
            error_fn: error_fn_rust,
            print_callback: Box::new(|string| print!("{}", string)),
            error_callback: Box::new(|string| write!(&mut std::io::stderr(), "{}", string).unwrap()),
            last_error: None,
        });
        debug_assert_eq!(std::mem::size_of::<Box<VmData>>(), std::mem::size_of::<SQUserPointer>());
        unsafe {
            sq_setforeignptr(vm, std::mem::transmute_copy::<Box<VmData>, SQUserPointer>(&data));
            let print_helper = std::mem::transmute::<_, SQPRINTFUNCTION>(squirrel_print_helper);
            let error_helper = std::mem::transmute::<_, SQPRINTFUNCTION>(squirrel_error_helper);
            sq_setprintfunc(vm, print_helper, error_helper);
            sq_setcompilererrorhandler(vm, Some(compiler_error_handler));
        }
        let mut vm = Vm(vm, data);
        vm.new_closure(exception_handler, 0);
        unsafe {
            sq_seterrorhandler(vm.0);
        }
        vm
    }

    #[doc(hidden)]
    pub unsafe fn from_raw(vm: HSQUIRRELVM) -> Vm {
        Vm(vm, std::mem::transmute::<SQUserPointer, Box<VmData>>(sq_getforeignptr(vm)))
    }

    /// Compile a Squirrel program. If successful the compiled program is pushed on the stack as a
    /// function.
    pub fn compile<R: Read>(&mut self, reader: &mut R, sourcename: &str) -> Result<(), Error> {
        let sourcename = try!(CString::new(sourcename));
        if unsafe {
            sq_compile(self.0,
                       Some(readfunc::<R>), reader as *mut R as SQUserPointer,
                       sourcename.as_ptr(), 1)
        } >= 0 {
            Ok(())
        } else {
            Err(self.1.last_error.take().unwrap_or(Error::Vm))
        }
    }

    pub fn push_root_table(&mut self) {
        unsafe { sq_pushroottable(self.0) }
    }

    pub fn call(&mut self, num_params: SQInteger) -> Result<Object, Error> {
        if unsafe { sq_call(self.0, num_params, true as SQBool, true as SQBool) } >= 0 {
            convert_object(self.0, -1)
        } else {
            Err(self.1.last_error.take().unwrap_or(Error::Vm))
        }
    }

    pub fn pop(&mut self, n: SQInteger) {
        unsafe { sq_pop(self.0, n) }
    }

    pub fn new_closure(&mut self, f: extern fn(HSQUIRRELVM) -> SQInteger, num_free_vars: SQUnsignedInteger) {
        unsafe {
            sq_newclosure(self.0, Some(f), num_free_vars)
        }
    }

    pub fn push_string(&mut self, string: &str) -> Result<(), Error> {
        let string = try!(CString::new(string));
        unsafe {
            sq_pushstring(self.0, string.as_ptr(), -1)
        };
        Ok(())
    }

    pub fn push(&mut self, idx: SQInteger) {
        unsafe {
            sq_push(self.0, idx)
        }
    }

    pub fn get(&mut self, idx: SQInteger) -> Result<(), Error> {
        if unsafe { sq_get(self.0, idx) } >= 0 {
            Ok(())
        } else {
            Err(Error::Vm)
        }
    }

    pub fn to_string(&mut self, idx: SQInteger) -> Result<(), Error> {
        if unsafe { sq_tostring(self.0, idx) } >= 0 {
            Ok(())
        } else {
            Err(Error::Vm)
        }
    }

    pub fn read_stack(&mut self, idx: SQInteger) -> Result<Object, Error> {
        convert_object(self.0, idx)
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        unsafe {
            sq_close(self.0)
        }
    }
}