#include <squirrel.h>

#include <sqstdaux.h>
#include <sqstdblob.h>
#include <sqstdio.h>
#include <sqstdmath.h>
#include <sqstdstring.h>
#include <sqstdsystem.h>

void squirrel_print_helper(HSQUIRRELVM vm, const SQChar *format, ...);
void squirrel_error_helper(HSQUIRRELVM vm, const SQChar *format, ...);