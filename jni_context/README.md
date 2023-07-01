# irondash_jni_context

This crate can be used to get access to JavaVM from Rust code.

It defines `JNI_OnLoad` function which is called by JVM when the library is loaded. It saves the pointer to JavaVM, which can be accessed later.

It also attempts to store the class loader that loaded Flutter application.

This only works if the dylib that uses this crate is loaded from Java using `System.loadLibrary`. It will not work if the dylib is loaded from other
code (i.e. through `dlopen`).

This crate also assumes that `System.loadLibrary` is called from main
thread. It will remember main thread looper and provides functionality
to schedule callbacks to be run on main thread.

#### Example usage

```rust
    let context = JniContext::get().unwrap();
    let java_vm = context.java_vm();
    let mut env = java_vm.attach_to_current_thread();

    // ...

    context.schedule_on_main_thread(|| {
        // This will be run on main thread
        println!("Hello from main thread!");
    });
```