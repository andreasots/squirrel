#include <squirrel.h>
#include <stdarg.h>
#include <stdio.h>

struct rust_data {
    void (*print_callback)(struct rust_data*, const SQChar*);
    void (*error_callback)(struct rust_data*, const SQChar*);
};

extern "C" void squirrel_print_helper(HSQUIRRELVM vm, const SQChar *format, ...) {
    SQChar buffer[4096];
    struct rust_data *ptr;
    va_list va;

    ptr = ((struct rust_data *)sq_getforeignptr(vm));
    if (!ptr || !ptr->print_callback)
        return;

    va_start(va, format);
    scvsprintf(buffer, 4096, format, va);
    va_end(va);

    return ptr->print_callback(ptr, buffer);
}

extern "C" void squirrel_error_helper(HSQUIRRELVM vm, const SQChar *format, ...) {
    SQChar buffer[4096];
    struct rust_data *ptr;
    va_list va;

    ptr = ((struct rust_data *)sq_getforeignptr(vm));
    if (!ptr || !ptr->error_callback)
        return;

    va_start(va, format);
    scvsprintf(buffer, 4096, format, va);
    va_end(va);

    return ptr->error_callback(ptr, buffer);
}