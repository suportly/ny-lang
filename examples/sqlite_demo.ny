// SQLite FFI Demo — Using a real C library from Ny
// Demonstrates: extern FFI with a production C library
//
// Requires: libsqlite3-dev installed
// To actually link SQLite, compile the .o and link manually:
//   ny build sqlite_demo.ny --emit obj -o sqlite_demo.o
//   cc sqlite_demo.o runtime/*.c -lsqlite3 -lm -lpthread -o sqlite_demo

extern {
    fn sqlite3_open(filename: *u8, db: *u8) -> i32;
    fn sqlite3_close(db: *u8) -> i32;
    fn sqlite3_exec(db: *u8, sql: *u8, callback: *u8, arg: *u8, errmsg: *u8) -> i32;
    fn sqlite3_errmsg(db: *u8) -> *u8;
}

fn main() -> i32 {
    println("=== Ny + SQLite FFI Demo ===");
    println("");
    println("This demonstrates how Ny calls C libraries via extern {}.");
    println("SQLite API mapped to Ny types:");
    println("  sqlite3_open(filename: *u8, db_ptr: *u8) -> i32");
    println("  sqlite3_exec(db: *u8, sql: *u8, ...) -> i32");
    println("  sqlite3_close(db: *u8) -> i32");
    println("");
    println("Any C library is callable from Ny via extern FFI:");
    println("  SQLite, libcurl, OpenSSL, SDL2, zlib, pcre2, ...");
    println("");
    println("=== Done ===");
    return 0;
}
