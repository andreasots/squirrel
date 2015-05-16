extern crate gcc;
extern crate bindgen;

fn main() {
    let mut src_dir = std::path::PathBuf::from("src/SQUIRREL3");
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);

    let mut clang_args = Vec::new();

    let mut config = gcc::Config::new();
    config.cpp(true);
    if std::env::var_os("CARGO_FEATURE_NO_GARBAGE_COLLECTOR").is_some() {
        config.define("NO_GARBAGE_COLLECTOR", None);
        clang_args.push(Into::into("-DNO_GARBAGE_COLLECTOR"));
    }
    if std::env::var_os("CARGO_FEATURE_UTF_16").is_some() {
        config.define("SQUNICODE", None);
        clang_args.push(Into::into("-DSQUNICODE"));
    }
    if std::env::var_os("CARGO_FEATURE_NO_COMPILER").is_some() {
        config.define("NO_COMPILER", None);
        clang_args.push(Into::into("-DNO_COMPILER"));
    }
    if std::env::var_os("CARGO_FEATURE_USE_DOUBLE").is_some() {
        config.define("SQUSEDOUBLE", None);
        clang_args.push(Into::into("-DSQUSEDOUBLE"));
    }

    src_dir.push("include");
    config.include(&src_dir);
    clang_args.push(format!("-I{}", src_dir.to_str().unwrap()));
    src_dir.pop();

    clang_args.push(Into::into("src/header.h"));

    for lib in ["squirrel", "sqstdlib"].into_iter() {
        src_dir.push(lib);
        for file in std::fs::read_dir(&src_dir).unwrap() {
            let path = file.unwrap().path();
            println!("{}", path.to_str().unwrap());
            if path.extension().and_then(std::ffi::OsStr::to_str) == Some("cpp") {
                config.file(path);
            }
        }
        src_dir.pop();
    }
    config.file("src/squirrel_print_helper.cpp");

    config.compile("libsquirrel.a");

    let mut options: bindgen::BindgenOptions = Default::default();
    options.clang_args = clang_args;
    let bindings = bindgen::Bindings::generate(&options, None, None).unwrap();
    bindings.write(Box::new(std::fs::File::create(out_dir.join("ffi.rs")).unwrap())).unwrap();
}
