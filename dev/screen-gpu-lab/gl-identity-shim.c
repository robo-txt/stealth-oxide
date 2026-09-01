/*
 * Experimental, intentionally incomplete native GPU identity shim.
 *
 * It keeps Mesa/LLVMpipe responsible for rendering and only replaces the
 * native GL identity strings.  This is a diagnostic prototype, not a
 * production browser patch: extensions, limits, precision, and pixels remain
 * those of LLVMpipe and are expected to expose contradictions.
 */
#define _GNU_SOURCE

#include <dlfcn.h>
#include <stddef.h>
#include <string.h>

typedef unsigned int GLenum;
typedef unsigned char GLubyte;
typedef const GLubyte *(*gl_get_string_fn)(GLenum name);
typedef const GLubyte *(*gl_get_string_i_fn)(GLenum name, unsigned int index);
typedef void *(*get_proc_address_fn)(const char *name);

const GLubyte *glGetString(GLenum name);
const GLubyte *glGetStringi(GLenum name, unsigned int index);

static const GLubyte amd_vendor[] = "AMD";
static const GLubyte amd_renderer[] = "AMD Radeon HD 3200 Graphics";

static gl_get_string_fn real_gl_get_string(void) {
    static gl_get_string_fn fn;
    if (!fn) {
        fn = (gl_get_string_fn)dlsym(RTLD_NEXT, "glGetString");
    }
    return fn;
}

static gl_get_string_i_fn real_gl_get_string_i(void) {
    static gl_get_string_i_fn fn;
    if (!fn) {
        fn = (gl_get_string_i_fn)dlsym(RTLD_NEXT, "glGetStringi");
    }
    return fn;
}

static void *real_get_proc_address(const char *name, const char *symbol) {
    static void *(*fn)(const char *);
    if (!fn) {
        fn = (void *(*)(const char *))dlsym(RTLD_NEXT, symbol);
    }
    return fn ? fn(name) : NULL;
}

static void *identity_proc(const char *name) {
    if (!name) {
        return NULL;
    }
    if (strcmp(name, "glGetString") == 0) {
        return (void *)&glGetString;
    }
    if (strcmp(name, "glGetStringi") == 0) {
        return (void *)&glGetStringi;
    }
    return NULL;
}

const GLubyte *glGetString(GLenum name) {
    switch (name) {
    case 0x1F00: /* GL_VENDOR */
        return amd_vendor;
    case 0x1F01: /* GL_RENDERER */
        return amd_renderer;
    default: {
        gl_get_string_fn fn = real_gl_get_string();
        return fn ? fn(name) : NULL;
    }
    }
}

const GLubyte *glGetStringi(GLenum name, unsigned int index) {
    gl_get_string_i_fn fn = real_gl_get_string_i();
    return fn ? fn(name, index) : NULL;
}

void *eglGetProcAddress(const char *name) {
    void *replacement = identity_proc(name);
    if (replacement) {
        return replacement;
    }
    return real_get_proc_address(name, "eglGetProcAddress");
}

void *glXGetProcAddress(const unsigned char *name) {
    void *replacement = identity_proc((const char *)name);
    if (replacement) {
        return replacement;
    }
    return real_get_proc_address((const char *)name, "glXGetProcAddress");
}

void *glXGetProcAddressARB(const unsigned char *name) {
    void *replacement = identity_proc((const char *)name);
    if (replacement) {
        return replacement;
    }
    return real_get_proc_address((const char *)name, "glXGetProcAddressARB");
}
